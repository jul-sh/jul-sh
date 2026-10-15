//! Keep the checked-in Kerberos subsets in sync with the site's copy.
//!
//! The coverage ranges in Kerberos-source.json are extracted from the fonts'
//! cmap when the subsets are made. Hashing the binaries here ensures that a
//! replaced font cannot silently keep an obsolete coverage manifest.

use eyre::{Result, WrapErr, bail, ensure, eyre};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::Path};

pub struct FontCoverage {
    ranges: Vec<(u32, u32)>,
}

impl FontCoverage {
    pub fn load(manifest_path: impl AsRef<Path>) -> Result<Self> {
        let manifest_path = manifest_path.as_ref();
        let manifest: Value = serde_json::from_slice(&fs::read(manifest_path)?)?;
        let font_dir = manifest_path
            .parent()
            .ok_or_else(|| eyre!("font manifest has no parent directory"))?;

        let fonts = manifest["fonts"]
            .as_array()
            .ok_or_else(|| eyre!("font manifest has no fonts array"))?;
        ensure!(!fonts.is_empty(), "font manifest has no fonts");
        for font in fonts {
            let file = font["file"]
                .as_str()
                .ok_or_else(|| eyre!("font manifest entry is missing file"))?;
            let expected_hash = font["sha256"]
                .as_str()
                .ok_or_else(|| eyre!("font manifest entry is missing sha256"))?;
            let bytes = fs::read(font_dir.join(file))
                .wrap_err_with(|| format!("cannot read font {file}"))?;
            let actual_hash = format!("{:x}", Sha256::digest(&bytes));
            ensure!(
                actual_hash == expected_hash,
                "{file} differs from Kerberos-source.json; regenerate the coverage ranges from the font's cmap"
            );
        }

        let listed_ranges = manifest["subset_codepoint_ranges"]
            .as_array()
            .ok_or_else(|| eyre!("font manifest has no subset_codepoint_ranges"))?;
        let mut ranges = Vec::with_capacity(listed_ranges.len());
        for listed in listed_ranges {
            let listed = listed
                .as_str()
                .ok_or_else(|| eyre!("font coverage range must be a string"))?;
            let (start, end) = listed.split_once('-').unwrap_or((listed, listed));
            let start = u32::from_str_radix(start, 16)
                .wrap_err_with(|| format!("invalid font coverage range {listed}"))?;
            let end = u32::from_str_radix(end, 16)
                .wrap_err_with(|| format!("invalid font coverage range {listed}"))?;
            ensure!(
                start <= end && char::from_u32(start).is_some() && char::from_u32(end).is_some(),
                "invalid font coverage range {listed}"
            );
            if let Some((_, previous_end)) = ranges.last() {
                ensure!(
                    start > *previous_end,
                    "font coverage ranges overlap or are unsorted"
                );
            }
            ranges.push((start, end));
        }
        ensure!(!ranges.is_empty(), "font coverage ranges are empty");
        Ok(Self { ranges })
    }

    pub fn check_page(&self, page_path: &Path, html: &str) -> Result<()> {
        // Include title, accessibility labels, and visible text. HTML markup and
        // asset paths are ASCII; checking them too is harmless. Comments are
        // excluded because they can contain notes or decorative emoji.
        let mut without_comments = String::with_capacity(html.len());
        let mut remaining = html;
        while let Some(start) = remaining.find("<!--") {
            without_comments.push_str(&remaining[..start]);
            remaining = &remaining[start + 4..];
            let end = remaining
                .find("-->")
                .ok_or_else(|| eyre!("unclosed HTML comment in {}", page_path.display()))?;
            remaining = &remaining[end + 3..];
        }
        without_comments.push_str(remaining);

        let decoded = html_escape::decode_html_entities(&without_comments);
        let missing: BTreeSet<char> = decoded
            .chars()
            .filter(|&c| !matches!(c, '\n' | '\r' | '\t'))
            .filter(|&c| {
                let cp = u32::from(c);
                !self
                    .ranges
                    .iter()
                    .any(|&(start, end)| start <= cp && cp <= end)
            })
            .collect();
        if !missing.is_empty() {
            let details = missing
                .iter()
                .take(8)
                .map(|&c| format!("U+{:04X} ({c})", u32::from(c)))
                .collect::<Vec<_>>()
                .join(", ");
            bail!(
                "{} needs characters absent from the Kerberos subset: {details}. Regenerate the font subsets and their coverage manifest.",
                page_path.display()
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::FontCoverage;
    use std::path::Path;

    #[test]
    fn detects_new_copy_even_when_written_as_html_entity() {
        let coverage = FontCoverage {
            ranges: vec![(0x20, 0x7e)],
        };
        assert!(
            coverage
                .check_page(Path::new("index.html"), "<p>Hi</p><!-- 🗽 -->")
                .is_ok()
        );
        let error = coverage
            .check_page(Path::new("index.html"), "<p>Hi &#x03A9;</p>")
            .unwrap_err()
            .to_string();
        assert!(error.contains("U+03A9"), "{error}");
    }
}
