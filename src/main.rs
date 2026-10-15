//! Minimal static-site generator: convert *.md → HTML, mirror ./static.

use eyre::Result;
use pulldown_cmark::{Event, Parser, Tag, html};
use std::{
    collections::{HashSet, hash_map::DefaultHasher},
    fs,
    hash::{Hash, Hasher},
    io,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

// Output the generated static site here.
static OUTPUT_DIR: &str = "./build";
// Copy this static directory into output. Used for font files, images, etc.
static STATIC_DIR: &str = "./static";
// Don't look for markdown files in these directories.
static IGNORED_MD_DIRECTORIES: &[&str] = &[
    "./target",
    "./.git",
    "./src",
    "./explorations",
    STATIC_DIR,
    OUTPUT_DIR,
];
enum PageKind {
    About,
    Cv,
    Regular { title: String },
}

fn main() -> Result<()> {
    if let Err(e) = fs::remove_dir_all(OUTPUT_DIR) {
        if e.kind() != io::ErrorKind::NotFound {
            return Err(e.into());
        }
    }
    fs::create_dir_all(OUTPUT_DIR)?;

    // Copy static assets preserving relative structure into build/static.
    let static_out = Path::new(OUTPUT_DIR).join("static");
    fs::create_dir_all(&static_out)?;
    let static_out_str = static_out.to_string_lossy();
    copy_dir_tree(STATIC_DIR, &static_out_str)?;

    // Discover all markdown files.
    let md_files: HashSet<PathBuf> = get_markdown_files(WalkDir::new("./").into_iter())?;

    // Convert each markdown file.
    for md in &md_files {
        let markdown = fs::read_to_string(md)?;
        let events = transform_events(md, &markdown, &md_files)?;
        let mut html_fragment = String::new();
        html::push_html(&mut html_fragment, events.into_iter());

        let rel_html = make_html_path_rel(md);
        let out_path = Path::new(OUTPUT_DIR).join(&rel_html);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&out_path, html_page(&rel_html, html_fragment)?)?;
    }

    Ok(())
}

// ---------- helpers ----------

/// Return a set of markdown file paths discovered from a WalkDir iterator,
/// skipping directories configured in `IGNORED_MD_DIRECTORIES`.
fn get_markdown_files(iter: walkdir::IntoIter) -> Result<HashSet<PathBuf>> {
    Ok(iter
        .filter_entry(|e| {
            !IGNORED_MD_DIRECTORIES
                .iter()
                .any(|ignore| e.path().starts_with(ignore))
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|e| e.into_path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("md"))
        .collect::<HashSet<_>>())
}

fn is_readme(p: &Path) -> bool {
    p.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.eq_ignore_ascii_case("readme.md"))
        .unwrap_or(false)
}

/// Copy a directory tree from `src` into `dst`, preserving the relative layout.
fn copy_dir_tree(src: &str, dst: &str) -> Result<()> {
    let src = Path::new(src).canonicalize()?;
    let dst = Path::new(dst);

    for entry in WalkDir::new(&src).into_iter() {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(&src).unwrap(); // safe by construction
        let target = dst.join(rel);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, &target)?;
        }
    }
    Ok(())
}

/// Return the output relative HTML path for a given markdown file.
/// `README.md` → `index.html` in the same directory; otherwise `foo.md` → `foo.html`.
fn make_html_path_rel(md: &Path) -> PathBuf {
    let mut rel = md.strip_prefix("./").unwrap_or(md).to_path_buf();
    if is_readme(md) {
        rel.set_file_name("index");
    }
    rel.set_extension("html");
    rel
}

/// Build the full HTML page around a fragment.
fn html_page(html_rel_path: &Path, fragment: String) -> Result<String> {
    let depth = html_rel_path.parent().map_or(0, |p| p.components().count());
    let prefix = "../".repeat(depth);
    let static_root = format!("{prefix}static");
    let about_link = format!("{prefix}index.html");
    let stylesheet = asset_url(&static_root, "main.css")?;
    let favicon = asset_url(&static_root, "favicon-32x32.png")?;
    let favicon_svg = asset_url(&static_root, "favicon.svg")?;
    let apple_touch_icon = asset_url(&static_root, "apple-touch-icon.png")?;
    let (title, body_class, contents, after_main) = match PageKind::try_from(html_rel_path)? {
        PageKind::About => (
            "Juliette Pluto".to_string(),
            "home variant-inline",
            // Keep README links usable on GitHub and point them at generated pages here.
            about_fragment(fragment.replace("href=\"CV.md", "href=\"CV.html"))?,
            format!(
                r#"<div id="image-preview" class="image-preview" aria-hidden="true" hidden><img alt=""></div><script src="{}" defer></script>"#,
                asset_url(&static_root, "preview.js")?
            ),
        ),
        PageKind::Cv => (
            "CV — Juliette Pluto".to_string(),
            "cv variant-inline",
            format!(
                r#"<article class="resume">{}</article>"#,
                cv_fragment(fragment)
            ),
            format!(r#"<footer class="cv-footer"><a href="{about_link}">← About</a></footer>"#),
        ),
        PageKind::Regular { title } => (
            title,
            "cv variant-inline",
            format!(r#"<article class="resume">{fragment}</article>"#),
            format!(r#"<footer class="cv-footer"><a href="{about_link}">← About</a></footer>"#),
        ),
    };
    let contents = add_illustration_animations(contents, &static_root)?;
    Ok(format!(
        r##"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width,initial-scale=1,shrink-to-fit=no"/>
    <title>{title}</title>
    <meta name="description" content="Juliette Pluto, Staff Software Engineer at Google DeepMind in New York City. Adversarial robustness for frontier AI systems."/>
    <link rel="stylesheet" href="{stylesheet}"/>
    <link rel="preload" href="{static_root}/Kerberos[wght].woff2?v=1.2.4" as="font" type="font/woff2" crossorigin/>
    <link rel="preload" href="{static_root}/IosevkaCharon-Medium.woff2?v=34.802" as="font" type="font/woff2" crossorigin/>
    <link rel="apple-touch-icon" sizes="180x180" href="{apple_touch_icon}"/>
    <link rel="icon" type="image/png" sizes="32x32" href="{favicon}"/>
    <link rel="icon" type="image/svg+xml" href="{favicon_svg}"/>
    <meta name="theme-color" content="#fcf8fc"/>
  </head>
  <body class="{body_class}">
    <a class="skip" href="#main">Skip to content</a>
    <main id="main">{contents}</main>
    {after_main}
  </body>
  <!-- 🗽 -->
</html>
"##
    ))
}

/// Fingerprint mutable assets so rebuilding also refreshes cached CSS and JS.
fn asset_url(static_root: &str, name: &str) -> Result<String> {
    let mut hash = DefaultHasher::new();
    fs::read(Path::new(STATIC_DIR).join(name))?.hash(&mut hash);
    Ok(format!("{static_root}/{name}?v={:x}", hash.finish()))
}

/// Keep the README readable as prose; the hidden markers position site-only art.
fn about_fragment(fragment: String) -> Result<String> {
    let mut paragraphs = Vec::new();
    let mut rest = fragment.trim();
    while !rest.is_empty() {
        let paragraph = rest
            .strip_prefix("<p>")
            .ok_or_else(|| eyre::eyre!("README must contain only paragraphs"))?;
        let (text, remaining) = paragraph
            .split_once("</p>")
            .ok_or_else(|| eyre::eyre!("unclosed README paragraph"))?;
        paragraphs.push(text.to_string());
        rest = remaining.trim();
    }
    if paragraphs.is_empty() {
        return Err(eyre::eyre!("README is empty"));
    }

    let mut introduction = paragraphs.remove(0);
    let asterism = r#"<span class="asterism" aria-hidden="true"><span>*</span><span>*</span><span>*</span></span>"#;
    for paragraph in paragraphs {
        let (lead, remainder) = paragraph
            .split_once("<!-- section -->")
            .ok_or_else(|| eyre::eyre!("README paragraph needs a section marker"))?;
        introduction.push_str(&format!(
            r#" <span class="section-start">{asterism} {lead}</span>{remainder}"#
        ));
    }

    replace_readme_marker(
        &mut introduction,
        "<strong>Juliette Pluto</strong><!-- portrait -->",
        r#"<span class="name">Juliette Pluto</span><button class="inline-portrait" type="button" data-portrait-zoom aria-label="Enlarge portrait of Juliette Pluto" aria-expanded="false" aria-controls="image-preview"><img src="static/portrait.jpg" alt="" width="666" height="1000" fetchpriority="high"></button>"#,
    )?;
    replace_readme_marker(
        &mut introduction,
        "New York City.<!-- liberty -->",
        r#"<span class="place">New York City.<span class="image-wrap liberty" aria-hidden="true"><svg class="inline-art" viewBox="185 2 491 695" preserveAspectRatio="xMidYMid meet" focusable="false"><image href="static/woodcut-cities.png" width="2172" height="724"/></svg></span></span>"#,
    )?;
    replace_readme_marker(
        &mut introduction,
        "London<!-- bus -->",
        r#"<span class="place">London<span class="image-wrap bus" aria-hidden="true"><img class="inline-art" src="static/london-bus.png" alt=""></span></span>"#,
    )?;
    replace_readme_marker(
        &mut introduction,
        "Berlin.<!-- bear -->",
        r#"<span class="place">Berlin.<span class="image-wrap bear" aria-hidden="true"><svg class="inline-art" viewBox="1603 87 401 612" preserveAspectRatio="xMidYMid meet" focusable="false"><image href="static/woodcut-cities.png" width="2172" height="724"/></svg></span></span>"#,
    )?;
    replace_readme_marker(
        &mut introduction,
        r#"<a href="CV.html">my CV</a>"#,
        r#"<a class="cv-link" href="CV.html">my CV</a>"#,
    )?;

    Ok(format!(
        "<h1 class=\"visually-hidden\">Juliette Pluto</h1><p class=\"introduction\">{introduction}</p>\n"
    ))
}

fn replace_readme_marker(html: &mut String, marker: &str, replacement: &str) -> Result<()> {
    if html.matches(marker).count() != 1 {
        return Err(eyre::eyre!(
            "expected one README marker {marker:?}"
        ));
    }
    *html = html.replacen(marker, replacement, 1);
    Ok(())
}

/// Animate the original bear and bus engravings, plus Liberty's flame.
fn add_illustration_animations(mut contents: String, static_root: &str) -> Result<String> {
    let bear_opening = r#"<span class="image-wrap bear" aria-hidden="true">"#;
    if contents.contains(bear_opening) {
        // The same engraved head is present at rest and in motion; no artwork swap.
        let head = "M1725 80H1960V236L1924 227L1896 217L1876 210L1853 219L1830 235L1803 245H1768V224H1725Z";
        let jaw = "M1725 140H1774V163L1811 163L1829 168L1840 177L1850 192L1844 210L1818 218H1768V210H1725Z";
        let paw =
            "M1598 137H1710V229L1775 252L1768 264L1760 284L1755 304L1752 325L1680 291L1598 269Z";
        // Leave a little original neck beneath the moving head to keep the join closed.
        let body_cut = "M1725 80H1960V218L1924 209L1896 199L1876 192L1853 201L1830 217L1803 227H1768V206H1725Z";
        let bear = format!(
            r#"<svg class="inline-art art-animation" viewBox="1603 87 401 612" preserveAspectRatio="xMidYMid meet" focusable="false">
<defs>
<clipPath id="bear-body"><path clip-rule="evenodd" d="M1603 87H2004V699H1603Z {body_cut} {paw}"/></clipPath>
<clipPath id="bear-upper-head"><path clip-rule="evenodd" d="{head} {jaw}"/></clipPath>
<clipPath id="bear-lower-jaw"><path d="{jaw}"/></clipPath>
<clipPath id="bear-upper-paw"><path d="{paw}"/></clipPath>
</defs>
<rect x="1603" y="87" width="401" height="612" fill="transparent" pointer-events="all"/>
<image href="{static_root}/woodcut-cities.png" width="2172" height="724" clip-path="url(#bear-body)"/>
<g class="bear-paw"><image href="{static_root}/woodcut-cities.png" width="2172" height="724" clip-path="url(#bear-upper-paw)"/></g>
<g class="bear-head">
<image href="{static_root}/woodcut-cities.png" width="2172" height="724" clip-path="url(#bear-upper-head)"/>
<g class="bear-jaw"><image href="{static_root}/woodcut-cities.png" width="2172" height="724" clip-path="url(#bear-lower-jaw)"/></g>
</g></svg>"#
        );
        contents = contents.replace(bear_opening, &format!("{bear_opening}{bear}"));
    }
    let opening = r#"<span class="image-wrap liberty" aria-hidden="true">"#;
    if contents.contains(opening) {
        let sprite = asset_url(static_root, "animations/liberty-flame-15.webp")?;
        let animated = format!(
            r#"<svg class="inline-art art-animation" viewBox="185 2 491 695" preserveAspectRatio="xMidYMid meet" focusable="false"><svg x="185" y="88" width="491" height="609" viewBox="185 88 491 609" overflow="hidden"><image href="{static_root}/woodcut-cities.png" width="2172" height="724"/></svg><svg x="192" y="0" width="142" height="94" viewBox="0 0 128 96" preserveAspectRatio="xMidYMax meet" overflow="hidden"><image class="animation-strip" href="{sprite}" width="1920" height="96"/></svg></svg>"#
        );
        contents = contents.replace(opening, &format!("{opening}{animated}"));
    }
    let bus_image = r#"<img class="inline-art" src="static/london-bus.png" alt="">"#;
    if contents.contains(bus_image) {
        let source = asset_url(static_root, "london-bus.png")?;
        // Every moving layer samples the same original image. At rest they align pixel for pixel.
        let bus = format!(
            r##"<svg class="inline-art bus-engraving" viewBox="0 0 1536 1024" preserveAspectRatio="xMidYMid meet" focusable="false" aria-hidden="true">
<defs>
<clipPath id="bus-rear-wheel-clip"><ellipse cx="253" cy="872" rx="25" ry="49"/></clipPath>
<clipPath id="bus-front-wheel-clip"><ellipse cx="800" cy="872" rx="32" ry="52"/></clipPath>
<radialGradient id="bus-headlight-glow"><stop offset="0" stop-color="#fff9d8" stop-opacity=".85"/><stop offset=".55" stop-color="#fff0aa" stop-opacity=".42"/><stop offset="1" stop-color="#fff0aa" stop-opacity="0"/></radialGradient>
</defs>
<image href="{source}" width="1536" height="1024"/>
<g clip-path="url(#bus-rear-wheel-clip)"><image class="bus-wheel bus-wheel-rear" href="{source}" width="1536" height="1024"/></g>
<g clip-path="url(#bus-front-wheel-clip)"><image class="bus-wheel bus-wheel-front" href="{source}" width="1536" height="1024"/></g>
<g class="bus-lights"><ellipse cx="1099" cy="842" rx="49" ry="53" fill="url(#bus-headlight-glow)"/><ellipse cx="1428" cy="841" rx="39" ry="49" fill="url(#bus-headlight-glow)"/></g>
</svg><svg class="inline-art art-halo bus-halo" viewBox="0 0 1536 1024" preserveAspectRatio="xMidYMid meet" focusable="false" aria-hidden="true"><defs><radialGradient id="bus-headlight-fog"><stop offset="0" stop-color="#fff9dd" stop-opacity=".95"/><stop offset=".3" stop-color="#f6d57b" stop-opacity=".58"/><stop offset=".7" stop-color="#eed092" stop-opacity=".4"/><stop offset="1" stop-color="#eed092" stop-opacity="0"/></radialGradient></defs><ellipse cx="1099" cy="842" rx="235" ry="220" fill="url(#bus-headlight-fog)"/><ellipse cx="1428" cy="841" rx="205" ry="205" fill="url(#bus-headlight-fog)"/></svg>"##
        );
        contents = contents.replace(bus_image, &bus);
    }
    Ok(contents)
}

/// Preserve the Markdown CV as the content source, adding its typographic structure.
fn cv_fragment(fragment: String) -> String {
    let mut cv = fragment
        .replace(
            "<h1>Juliette Pluto</h1>",
            r#"<h1><a href="index.html">Juliette Pluto</a></h1>"#,
        )
        .replace("<h4>", r#"<p class="dates">"#)
        .replace("</h4>", "</p>");
    for heading in ["Experience", "Papers", "Talks", "Education", "Recognition"] {
        cv = cv.replace(
            &format!("<h2>{heading}</h2>"),
            &format!(r#"<h2 id="{}">{heading}</h2>"#, heading.to_lowercase()),
        );
    }
    for (needle, text, city, viewbox) in [
        ("· NYC ·", "NYC", "liberty", "185 2 491 695"),
        (
            "2018-01 — 2020-03 · Berlin",
            "Berlin",
            "bear",
            "1603 87 401 612",
        ),
    ] {
        let art = format!(
            r#"<span class="place">{text}<span class="image-wrap {city}" aria-hidden="true"><svg class="inline-art" viewBox="{viewbox}" preserveAspectRatio="xMidYMid meet" focusable="false"><image href="static/woodcut-cities.png" width="2172" height="724"/></svg></span></span>"#
        );
        cv = cv.replace(needle, &needle.replace(text, &art));
    }
    cv
}

impl TryFrom<&Path> for PageKind {
    type Error = eyre::Report;

    fn try_from(html_rel_path: &Path) -> Result<Self> {
        let ext = html_rel_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if ext != "html" {
            return Err(eyre::eyre!(
                "expected .html path for page kind, got {}",
                html_rel_path.display()
            ));
        }
        let stem = html_rel_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("index");
        match html_rel_path.to_str() {
            Some("index.html") => Ok(PageKind::About),
            Some("CV.html") => Ok(PageKind::Cv),
            _ => Ok(PageKind::Regular {
                title: format!("{stem} — Juliette Pluto"),
            }),
        }
    }
}

fn transform_events<'a>(
    current_md: &Path,
    markdown: &'a str,
    all_md: &HashSet<PathBuf>,
) -> Result<Vec<Event<'a>>> {
    let mut out = Vec::new();

    for ev in Parser::new(markdown) {
        match ev {
            // If a markdown link points to an existing .md in the project, rewrite to its .html path.
            Event::Start(Tag::Link(link_ty, mut dest, title)) => {
                if dest.ends_with(".md") {
                    // Resolve relative to current file directory.
                    let dest_string = dest.to_string();
                    let dest_rel = Path::new(&dest_string);
                    let full = current_md
                        .parent()
                        .unwrap_or(Path::new("./"))
                        .join(dest_rel);

                    if all_md.contains(&full) {
                        let html_rel = make_html_path_rel(dest_rel);
                        dest = html_rel.to_string_lossy().into_owned().into();
                    }
                }
                out.push(Event::Start(Tag::Link(link_ty, dest, title)));
            }
            other => out.push(other),
        }
    }
    Ok(out)
}
