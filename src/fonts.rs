//! Automated two-stage webfont builder from pinned upstream Kerberos variable fonts.
//!
//! Fetches and SHA-256-verifies upstream Kerberos WOFF2 release assets into
//! `./target/font-cache/upstream/`, extracts every rendered Unicode character
//! across the generated HTML pages, verifies full upstream `cmap` coverage, and
//! emits deterministic two-stage WOFF2 subsets into `./build/static/fonts/`:
//!
//! - **Boot cuts (`[opsz]`):** lightweight weight-pinned subsets for
//!   fast, zero-CLS first paint across all viewport optical sizes.
//! - **Progressive variable cuts (`[GRAD,opsz,wght]`):** loaded
//!   in the background after first paint via the CSS `FontFace` API.

use eyre::{Result, WrapErr, bail, ensure, eyre};
use hb_subset::{Blob, FontFace, SubsetInput, sys};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sigilbuzz_woff::{unwrap_woff2, wrap_woff2};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

const ALLOWED_RELEASE_URL_PREFIX: &str = "https://github.com/jul-sh/Kerberos/releases/download/";
const MAX_UPSTREAM_FONT_BYTES: u64 = 16 * 1024 * 1024;
const CACHE_ROOT: &str = "./target/font-cache";
const CACHE_SCHEMA_VERSION: &str = "v1-hb-subset-woff2";

#[derive(Debug, Deserialize)]
struct FontManifest {
    release: String,
    sources: Vec<UpstreamSource>,
}

#[derive(Debug, Clone, Deserialize)]
struct UpstreamSource {
    style: UpstreamStyle,
    source_url: String,
    sha256: String,
    bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpstreamStyle {
    Roman,
    Italic,
}

/// Type-safe variation instancing mode for each generated webfont cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariationMode {
    /// Fast first-paint cut: pins `wght` (plus `GRAD=0, YTLC=500`),
    /// keeping `opsz` variable so automatic optical sizing matches the full VF.
    BootOpszOnly { weight: u16 },
    /// Progressive Roman variable font (`wght`, `opsz`, `GRAD` kept variable).
    VariableRomanGrade,
    /// Progressive Italic variable font (`wght`, `opsz`, `GRAD` kept variable).
    VariableItalicGrade,
}

impl VariationMode {
    fn pinned_axes(self) -> Vec<([u8; 4], f32)> {
        match self {
            Self::BootOpszOnly { weight } => vec![
                (*b"wght", f32::from(weight)),
                (*b"GRAD", 0.0),
                (*b"YTLC", 500.0),
            ],
            Self::VariableRomanGrade => vec![(*b"YTLC", 500.0)],
            Self::VariableItalicGrade => vec![(*b"YTLC", 500.0)],
        }
    }

    fn cache_descriptor(self) -> String {
        match self {
            Self::BootOpszOnly { weight } => format!("boot-opsz-w{weight}"),
            Self::VariableRomanGrade => "var-roman-grade-opsz-wght".to_string(),
            Self::VariableItalicGrade => "var-italic-grade-opsz-wght".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FontArtifact {
    BootKerberosRegular,
    BootKerberosMedium,
    BootKerberosSemiBold,
    BootKerberosItalic,
    VariableRoman,
    VariableItalic,
}

impl FontArtifact {
    pub const ALL: [Self; 6] = [
        Self::BootKerberosRegular,
        Self::BootKerberosMedium,
        Self::BootKerberosSemiBold,
        Self::BootKerberosItalic,
        Self::VariableRoman,
        Self::VariableItalic,
    ];

    pub fn file_name(self) -> &'static str {
        match self {
            Self::BootKerberosRegular => "Kerberos-Boot-Regular.woff2",
            Self::BootKerberosMedium => "Kerberos-Boot-Medium.woff2",
            Self::BootKerberosSemiBold => "Kerberos-Boot-SemiBold.woff2",
            Self::BootKerberosItalic => "Kerberos-Boot-Italic.woff2",
            Self::VariableRoman => "Kerberos-VF-Roman.woff2",
            Self::VariableItalic => "Kerberos-VF-Italic.woff2",
        }
    }

    fn upstream_style(self) -> UpstreamStyle {
        match self {
            Self::BootKerberosItalic | Self::VariableItalic => UpstreamStyle::Italic,
            Self::BootKerberosRegular
            | Self::BootKerberosMedium
            | Self::BootKerberosSemiBold
            | Self::VariableRoman => UpstreamStyle::Roman,
        }
    }

    fn variation_mode(self) -> VariationMode {
        match self {
            Self::BootKerberosRegular => VariationMode::BootOpszOnly {
                weight: 400,
            },
            Self::BootKerberosMedium => VariationMode::BootOpszOnly {
                weight: 500,
            },
            Self::BootKerberosSemiBold => VariationMode::BootOpszOnly {
                weight: 550,
            },
            Self::BootKerberosItalic => VariationMode::BootOpszOnly {
                weight: 400,
            },
            Self::VariableRoman => VariationMode::VariableRomanGrade,
            Self::VariableItalic => VariationMode::VariableItalicGrade,
        }
    }
}

/// Which background variable fonts a rendered page needs after first paint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageFontUpgrade {
    RomanOnly,
    RomanAndItalic,
}

impl PageFontUpgrade {
    pub fn from_rendered_html(html: &str) -> Result<Self> {
        let stripped = strip_html_comments(Path::new("<fragment>"), html)?;
        if contains_italic_element(&stripped) {
            Ok(Self::RomanAndItalic)
        } else {
            Ok(Self::RomanOnly)
        }
    }
}

pub struct BuiltFonts {
    fingerprints: BTreeMap<FontArtifact, String>,
}

impl BuiltFonts {
    pub fn build(
        manifest_path: impl AsRef<Path>,
        pages: &[(PathBuf, String)],
        fonts_out_dir: impl AsRef<Path>,
    ) -> Result<Self> {
        let manifest_path = manifest_path.as_ref();
        let manifest: FontManifest = serde_json::from_slice(
            &fs::read(manifest_path)
                .wrap_err_with(|| format!("cannot read {}", manifest_path.display()))?,
        )
        .wrap_err_with(|| format!("invalid JSON in {}", manifest_path.display()))?;

        let sources = validate_manifest(&manifest)?;
        let cache_dir = Path::new(CACHE_ROOT);
        let upstream_cache = cache_dir.join("upstream");
        let subset_cache = cache_dir.join("subsets");
        fs::create_dir_all(&upstream_cache)?;
        fs::create_dir_all(&subset_cache)?;

        let roman_woff2 = load_or_fetch_upstream(&sources[&UpstreamStyle::Roman], &upstream_cache)?;
        let italic_woff2 =
            load_or_fetch_upstream(&sources[&UpstreamStyle::Italic], &upstream_cache)?;

        let roman_sfnt =
            unwrap_woff2(&roman_woff2).map_err(|e| eyre!("cannot decode Roman WOFF2: {e}"))?;
        let italic_sfnt =
            unwrap_woff2(&italic_woff2).map_err(|e| eyre!("cannot decode Italic WOFF2: {e}"))?;

        let site_chars = collect_and_verify_site_chars(pages, &roman_sfnt, &italic_sfnt)?;

        let fonts_out_dir = fonts_out_dir.as_ref();
        fs::create_dir_all(fonts_out_dir)?;

        // Variable-font instancing is memory intensive; bound concurrent workers so
        // a cold build does not hold all upstream instances in memory at once.
        let mut built = Vec::with_capacity(FontArtifact::ALL.len());
        for artifacts in FontArtifact::ALL.chunks(2) {
            built.extend(thread::scope(|scope| -> Result<Vec<_>> {
                let mut handles = Vec::with_capacity(artifacts.len());
                for &artifact in artifacts {
                    let style = artifact.upstream_style();
                    let sfnt = match style {
                        UpstreamStyle::Roman => roman_sfnt.as_slice(),
                        UpstreamStyle::Italic => italic_sfnt.as_slice(),
                    };
                    let upstream_sha = sources[&style].sha256.as_str();
                    let chars = &site_chars;
                    let subset_cache_ref = subset_cache.as_path();
                    let out_dir_ref = fonts_out_dir;

                    handles.push(scope.spawn(move || -> Result<(FontArtifact, String)> {
                        let woff2 = build_or_load_subset(
                            artifact,
                            sfnt,
                            upstream_sha,
                            chars,
                            subset_cache_ref,
                        )?;
                        let hash = short_sha256(&woff2);
                        let out_path = out_dir_ref.join(artifact.file_name());
                        fs::write(&out_path, &woff2)
                            .wrap_err_with(|| format!("cannot write {}", out_path.display()))?;
                        Ok((artifact, hash))
                    }));
                }

                let mut results = Vec::with_capacity(handles.len());
                for handle in handles {
                    let res = handle
                        .join()
                        .map_err(|_| eyre!("font subset worker panicked"))??;
                    results.push(res);
                }
                Ok(results)
            })?);
        }

        Ok(Self {
            fingerprints: built.into_iter().collect(),
        })
    }

    /// URL relative to a page's `static_root` (e.g. `static/fonts/foo.woff2?v=...`).
    pub fn page_url(&self, static_root: &str, artifact: FontArtifact) -> String {
        let hash = &self.fingerprints[&artifact];
        format!("{static_root}/fonts/{}?v={hash}", artifact.file_name())
    }

    /// `@font-face` rules for `build/static/main.css` (paths relative to `build/static/`).
    pub fn font_face_css(&self) -> String {
        let mut css = String::new();
        for (family, weight, style, artifact) in [
            ("Kerberos", 400, "normal", FontArtifact::BootKerberosRegular),
            ("Kerberos", 500, "normal", FontArtifact::BootKerberosMedium),
            ("Kerberos", 550, "normal", FontArtifact::BootKerberosSemiBold),
            ("Kerberos", 400, "italic", FontArtifact::BootKerberosItalic),
        ] {
            let hash = &self.fingerprints[&artifact];
            let file = artifact.file_name();
            css.push_str(&format!(
                "@font-face{{font-family:{family};src:url('./fonts/{file}?v={hash}') format('woff2');font-weight:{weight};font-style:{style};font-display:swap}}\n"
            ));
        }
        css
    }
}

fn validate_manifest(manifest: &FontManifest) -> Result<BTreeMap<UpstreamStyle, UpstreamSource>> {
    ensure!(
        !manifest.release.trim().is_empty(),
        "Kerberos-source.json release must not be empty"
    );
    let mut map = BTreeMap::new();
    for src in &manifest.sources {
        ensure!(
            src.source_url.starts_with(ALLOWED_RELEASE_URL_PREFIX),
            "disallowed upstream font URL {:?}; must start with {ALLOWED_RELEASE_URL_PREFIX}",
            src.source_url
        );
        ensure!(
            src.sha256.len() == 64
                && src
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "invalid lowercase hex sha256 {:?} in Kerberos-source.json",
            src.sha256
        );
        ensure!(
            src.bytes > 0 && (src.bytes as u64) <= MAX_UPSTREAM_FONT_BYTES,
            "upstream font byte size {} is out of bounds",
            src.bytes
        );
        ensure!(
            map.insert(src.style, src.clone()).is_none(),
            "duplicate {:?} entry in Kerberos-source.json",
            src.style
        );
    }
    for required in [UpstreamStyle::Roman, UpstreamStyle::Italic] {
        ensure!(
            map.contains_key(&required),
            "Kerberos-source.json is missing {:?} upstream source",
            required
        );
    }
    Ok(map)
}

fn load_or_fetch_upstream(src: &UpstreamSource, cache_dir: &Path) -> Result<Vec<u8>> {
    let cached_path = cache_dir.join(format!("{}.woff2", src.sha256));
    if cached_path.is_file() {
        let bytes = fs::read(&cached_path)?;
        if verify_bytes(&bytes, src.bytes, &src.sha256).is_ok() {
            return Ok(bytes);
        }
        let _ = fs::remove_file(&cached_path);
    }

    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
        .https_only(true)
        .build();
    let agent: ureq::Agent = config.into();
    let mut response = agent
        .get(&src.source_url)
        .call()
        .wrap_err_with(|| format!("failed to download upstream font {}", src.source_url))?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(MAX_UPSTREAM_FONT_BYTES)
        .read_to_vec()
        .wrap_err_with(|| format!("failed reading response body from {}", src.source_url))?;

    verify_bytes(&bytes, src.bytes, &src.sha256)
        .wrap_err_with(|| format!("integrity check failed for {}", src.source_url))?;

    write_atomic(&cached_path, &bytes)?;
    Ok(bytes)
}

fn verify_bytes(bytes: &[u8], expected_len: usize, expected_sha256: &str) -> Result<()> {
    ensure!(
        bytes.len() == expected_len,
        "expected {expected_len} bytes, got {}",
        bytes.len()
    );
    let actual = hex_sha256(bytes);
    ensure!(
        actual == expected_sha256,
        "SHA-256 mismatch: expected {expected_sha256}, got {actual}"
    );
    Ok(())
}

fn collect_and_verify_site_chars(
    pages: &[(PathBuf, String)],
    roman_sfnt: &[u8],
    italic_sfnt: &[u8],
) -> Result<BTreeSet<char>> {
    let roman_blob = Blob::from_bytes(roman_sfnt).map_err(|_| eyre!("invalid Roman blob"))?;
    let roman_face = FontFace::new(roman_blob).map_err(|_| eyre!("invalid Roman font face"))?;
    let roman_cov = roman_face
        .covered_codepoints()
        .map_err(|_| eyre!("cannot read Roman cmap"))?;

    let italic_blob = Blob::from_bytes(italic_sfnt).map_err(|_| eyre!("invalid Italic blob"))?;
    let italic_face = FontFace::new(italic_blob).map_err(|_| eyre!("invalid Italic font face"))?;
    let italic_cov = italic_face
        .covered_codepoints()
        .map_err(|_| eyre!("cannot read Italic cmap"))?;

    // Always include printable ASCII alongside every rendered character across all pages.
    let mut all_chars: BTreeSet<char> = (' '..='~').collect();

    for (page_path, html) in pages {
        let page_chars = extract_visible_chars(page_path, html)?;
        let missing: Vec<char> = page_chars
            .iter()
            .copied()
            .filter(|&c| !roman_cov.contains(c) || !italic_cov.contains(c))
            .collect();
        if !missing.is_empty() {
            let details = missing
                .into_iter()
                .take(8)
                .map(|c| format!("U+{:04X} ({c})", u32::from(c)))
                .collect::<Vec<_>>()
                .join(", ");
            bail!(
                "{} needs characters absent from upstream Kerberos: {details}",
                page_path.display()
            );
        }
        all_chars.extend(page_chars);
    }

    Ok(all_chars)
}

pub fn extract_visible_chars(page_path: &Path, html: &str) -> Result<BTreeSet<char>> {
    let without_comments = strip_html_comments(page_path, html)?;
    let decoded = html_escape::decode_html_entities(&without_comments);
    Ok(decoded
        .chars()
        .filter(|&c| !matches!(c, '\n' | '\r' | '\t'))
        .collect())
}

fn strip_html_comments(page_path: &Path, html: &str) -> Result<String> {
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
    Ok(without_comments)
}

fn contains_italic_element(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    lower.contains("<em>")
        || lower.contains("<em ")
        || lower.contains("<i>")
        || lower.contains("<i ")
}

fn build_or_load_subset(
    artifact: FontArtifact,
    sfnt: &[u8],
    upstream_sha256: &str,
    chars: &BTreeSet<char>,
    subset_cache_dir: &Path,
) -> Result<Vec<u8>> {
    let mode = artifact.variation_mode();
    let cache_key = compute_subset_cache_key(upstream_sha256, mode, chars);
    let cache_path = subset_cache_dir.join(format!("{cache_key}.woff2"));
    if cache_path.is_file() {
        return fs::read(&cache_path)
            .wrap_err_with(|| format!("cannot read cached subset {}", cache_path.display()));
    }

    let woff2 = subset_and_compress(sfnt, chars, mode)
        .wrap_err_with(|| format!("failed building font subset {}", artifact.file_name()))?;
    write_atomic(&cache_path, &woff2)?;
    Ok(woff2)
}

fn subset_and_compress(
    sfnt: &[u8],
    chars: &BTreeSet<char>,
    mode: VariationMode,
) -> Result<Vec<u8>> {
    let blob = Blob::from_bytes(sfnt).map_err(|_| eyre!("cannot allocate HarfBuzz blob"))?;
    let face = FontFace::new(blob).map_err(|_| eyre!("cannot parse SFNT font face"))?;
    let mut input = SubsetInput::new().map_err(|_| eyre!("cannot allocate HarfBuzz SubsetInput"))?;

    // Retain all OpenType layout features (ligatures, kerning, marks, figures) and name records.
    input.layout_feature_tag_set().insert_range(..);
    input.name_id_set().insert_range(..);
    input.name_lang_id_set().insert_range(..);
    input.flags().retain_notdef_outline().retain_legacy_names();

    {
        let mut unicode_set = input.unicode_set();
        for &ch in chars {
            unicode_set.insert(ch);
        }
    }

    for (tag_bytes, value) in mode.pinned_axes() {
        let tag = u32::from_be_bytes(tag_bytes);
        // SAFETY: `input.as_raw()` and `face.as_raw()` are valid live HarfBuzz pointers owned
        // in this stack frame, and `hb_subset_input_pin_axis_location` does not retain borrows.
        let ok = unsafe {
            sys::hb_subset_input_pin_axis_location(input.as_raw(), face.as_raw(), tag, value)
        };
        ensure!(
            ok != 0,
            "HarfBuzz failed to pin axis {:?} to {value}",
            std::str::from_utf8(&tag_bytes).unwrap_or("????")
        );
    }

    let subset_face = input
        .subset_font(&face)
        .map_err(|_| eyre!("HarfBuzz subset_font failed"))?;
    let subset_sfnt = subset_face.underlying_blob().to_vec();
    let woff2 =
        wrap_woff2(&subset_sfnt).map_err(|e| eyre!("WOFF2 compression failed: {e}"))?;
    Ok(woff2)
}

fn compute_subset_cache_key(
    upstream_sha256: &str,
    mode: VariationMode,
    chars: &BTreeSet<char>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(CACHE_SCHEMA_VERSION.as_bytes());
    hasher.update(b"\0");
    hasher.update(upstream_sha256.as_bytes());
    hasher.update(b"\0");
    hasher.update(mode.cache_descriptor().as_bytes());
    hasher.update(b"\0");
    for &ch in chars {
        hasher.update((ch as u32).to_be_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn write_atomic(target: &Path, bytes: &[u8]) -> Result<()> {
    let parent = target
        .parent()
        .ok_or_else(|| eyre!("path {} has no parent", target.display()))?;
    fs::create_dir_all(parent)?;
    let tmp = parent.join(format!(
        ".{}.tmp-{}",
        target
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("font"),
        std::process::id()
    ));
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, target)?;
    Ok(())
}

fn hex_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn short_sha256(bytes: &[u8]) -> String {
    let full = hex_sha256(bytes);
    full[..16].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_visible_chars_and_decodes_entities_ignoring_comments() {
        let chars = extract_visible_chars(
            Path::new("index.html"),
            "<p>Hi &#x03A9; &mdash;</p><!-- 🗽 -->",
        )
        .unwrap();
        assert!(chars.contains(&'\u{03A9}'));
        assert!(chars.contains(&'—'));
        assert!(!chars.contains(&'🗽'));
    }

    #[test]
    fn detects_italic_elements_only_outside_comments() {
        assert_eq!(
            PageFontUpgrade::from_rendered_html("<p>Hello</p><!-- <em>no</em> -->").unwrap(),
            PageFontUpgrade::RomanOnly
        );
        assert_eq!(
            PageFontUpgrade::from_rendered_html("<p><em>Paper Title</em></p>").unwrap(),
            PageFontUpgrade::RomanAndItalic
        );
    }

    #[test]
    fn rejects_untrusted_upstream_urls() {
        let bad = FontManifest {
            release: "v1.5.0-rc.1".to_string(),
            sources: vec![UpstreamSource {
                style: UpstreamStyle::Roman,
                source_url: "https://evil.example/Kerberos-VF.woff2".to_string(),
                sha256: "935edb1c9a4ad4043e55565ed3c54563f26853a07c36e8d1bd6e68eb3dc98c8c"
                    .to_string(),
                bytes: 100,
            }],
        };
        assert!(validate_manifest(&bad).is_err());
    }
}
