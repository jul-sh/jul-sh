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
    let (title, body_class, contents, after_main) = match PageKind::try_from(html_rel_path)? {
        PageKind::About => (
            "Juliette Pluto".to_string(),
            "home variant-inline",
            include_str!("about.html").to_string(),
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
    Ok(format!(
        r##"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width,initial-scale=1,shrink-to-fit=no"/>
    <title>{title}</title>
    <meta name="description" content="Juliette Pluto, Staff Software Engineer at Google DeepMind in New York City. Adversarial robustness for frontier AI systems."/>
    <link rel="stylesheet" href="{stylesheet}"/>
    <link rel="preload" href="{static_root}/Kerberos[wght].woff2?v=1.2.1" as="font" type="font/woff2" crossorigin/>
    <link rel="preload" href="{static_root}/IosevkaCharon-Bold.woff2" as="font" type="font/woff2" crossorigin/>
    <link rel="apple-touch-icon" sizes="180x180" href="{static_root}/apple-touch-icon.png"/>
    <link rel="icon" type="image/png" sizes="32x32" href="{static_root}/favicon-32x32.png"/>
    <meta name="theme-color" content="#eef1f0"/>
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
