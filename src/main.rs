//! Minimal static-site generator: convert *.md → HTML, mirror ./static.

use eyre::Result;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, html};
use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

// Output the generated static site here.
static OUTPUT_DIR: &str = "./build";
// Copy this static directory into output. Used for font files, images, etc.
static STATIC_DIR: &str = "./static";
// Don't look for markdown files in these directories.
static IGNORED_MD_DIRECTORIES: &[&str] = &["./target", "./.git", STATIC_DIR, OUTPUT_DIR];
enum PageKind {
    Index { title: String },
    Regular { title: String },
}

impl PageKind {
    fn title(self) -> String {
        match self {
            Self::Index { title } | Self::Regular { title } => title,
        }
    }
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

///// Build the full HTML page around a fragment.
fn html_page(html_rel_path: &Path, fragment: String) -> Result<String> {
    let title = PageKind::try_from(html_rel_path)?.title();
    let stem = html_rel_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("index");
    let is_index = stem == "index";
    let page_class = if is_index { "index" } else { "page" };
    let idx_current = if is_index { r#" aria-current="page""# } else { "" };
    let cv_current = if stem.eq_ignore_ascii_case("cv") {
        r#" aria-current="page""#
    } else {
        ""
    };
    let masthead = format!(
        r#"<header class="masthead"><a class="cell brand" href="index.html">jul.sh</a><span class="cell spacer"></span><nav class="masthead-nav"><a class="cell"{idx_current} href="index.html">Index</a><a class="cell"{cv_current} href="CV.html">CV</a></nav></header>"#
    );
    let cities_strip = if is_index {
        r#"<section class="cities-strip" aria-label="Cities">
        <div class="city-card">
          <div class="city-card-head"><span class="num">01</span><span>Berlin</span></div>
          <div class="city-art"><img src="./static/berlin.png" alt="Berlin bear illustration"/></div>
          <div class="city-card-foot"><span>52°31′N 13°24′E</span><span>2013 — 2020</span></div>
        </div>
        <div class="city-card">
          <div class="city-card-head"><span class="num">02</span><span>London</span></div>
          <div class="city-art"><img src="./static/london.png" alt="London double-decker bus illustration"/></div>
          <div class="city-card-foot"><span>51°30′N 0°07′W</span><span>2020 — 2022</span></div>
        </div>
        <div class="city-card">
          <div class="city-card-head"><span class="num">03</span><span>New York</span></div>
          <div class="city-art"><img src="./static/nyc.png" alt="Statue of Liberty illustration"/></div>
          <div class="city-card-foot"><span>40°43′N 74°00′W</span><span>2022 — Present</span></div>
        </div>
      </section>"#
    } else {
        ""
    };
    let footer = r#"<footer class="foot"><span class="cell">New York · London · Berlin</span><a class="cell" href="mailto:j@jul.sh">j@jul.sh</a></footer>"#;
    Ok(format!(
        r##"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width,initial-scale=1,shrink-to-fit=no"/>
    <title>{title}</title>
    <meta name="description" content="Staff Software Engineer at Google DeepMind"/>
    <link rel="stylesheet" href="./static/main.css"/>
    <link rel="preload" href="./static/Kerberos-Roman.woff2" as="font" type="font/woff2" crossorigin/>
    <link rel="preload" href="./static/Kerberos-Italic.woff2" as="font" type="font/woff2" crossorigin/>
    <link rel="preload" href="./static/IosevkaCharon-Bold.woff2" as="font" type="font/woff2" crossorigin/>
    <link rel="apple-touch-icon" sizes="180x180" href="./static/apple-touch-icon.png"/>
    <link rel="icon" type="image/png" sizes="32x32" href="./static/favicon-32x32.png"/>
    <meta name="theme-color" content="#ffffff"/>
  </head>
  <body class="{page_class}">
    <main>
      {masthead}
      {fragment}
      {cities_strip}
      {footer}
    </main>
  </body>
  <!-- 🗽 -->
</html>
"##
    ))
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
        if stem == "index" {
            Ok(PageKind::Index {
                title: "Juliette Pluto".to_string(),
            })
        } else {
            Ok(PageKind::Regular {
                title: format!("{stem} — Juliette Pluto"),
            })
        }
    }
}

fn transform_events<'a>(
    current_md: &Path,
    markdown: &'a str,
    all_md: &HashSet<PathBuf>,
) -> Result<Vec<Event<'a>>> {
    let mut out = Vec::new();
    let is_readme_file = is_readme(current_md);

    let mut in_intro = false;
    let mut in_section = false;
    let mut in_entry = false;
    let mut in_entry_head = false;
    let mut in_h4 = false;
    let mut section_idx: usize = 0;

    for ev in Parser::new(markdown) {
        // Close `<header class="entry-head">` before non-H4 elements start inside an entry.
        if in_entry_head && !in_h4 {
            let close_head = match &ev {
                Event::Start(Tag::Heading(HeadingLevel::H4, _, _)) => false,
                Event::Start(_) => true,
                _ => false,
            };
            if close_head {
                out.push(Event::Html("</header>".into()));
                in_entry_head = false;
            }
        }

        match ev {
            Event::Start(Tag::Heading(HeadingLevel::H1, id, classes)) => {
                out.push(Event::Html(r#"<div class="hero-block"><div class="hero-main">"#.into()));
                out.push(Event::Start(Tag::Heading(HeadingLevel::H1, id, classes)));
                in_intro = true;
            }
            Event::End(Tag::Heading(HeadingLevel::H1, id, classes)) => {
                out.push(Event::End(Tag::Heading(HeadingLevel::H1, id, classes)));
                if is_readme_file {
                    out.push(Event::Html(
                        r#"</div></div><div class="index-body">"#.into(),
                    ));
                }
            }
            Event::Start(Tag::Heading(HeadingLevel::H2, id, classes)) => {
                if in_entry_head {
                    out.push(Event::Html("</header>".into()));
                    in_entry_head = false;
                }
                if in_entry {
                    out.push(Event::Html("</article>".into()));
                    in_entry = false;
                }
                if in_section {
                    out.push(Event::Html("</div></section>".into()));
                }
                if in_intro {
                    if is_readme_file {
                        out.push(Event::Html("</div>".into()));
                    } else {
                        out.push(Event::Html(
                            r#"</div><div class="hero-stamps" aria-hidden="true"><img src="./static/berlin.png" alt=""/><img src="./static/london.png" alt=""/><img src="./static/nyc.png" alt=""/></div></div>"#.into(),
                        ));
                    }
                    in_intro = false;
                }
                section_idx += 1;
                out.push(Event::Html(
                    format!(
                        r#"<section class="block"><aside class="block-label"><span class="num">{section_idx:02}</span>"#
                    )
                    .into(),
                ));
                out.push(Event::Start(Tag::Heading(HeadingLevel::H2, id, classes)));
                in_section = true;
            }
            Event::End(Tag::Heading(HeadingLevel::H2, id, classes)) => {
                out.push(Event::End(Tag::Heading(HeadingLevel::H2, id, classes)));
                out.push(Event::Html(r#"</aside><div class="block-content">"#.into()));
            }
            Event::Start(Tag::Heading(HeadingLevel::H3, id, classes)) if in_section => {
                if in_entry_head {
                    out.push(Event::Html("</header>".into()));
                }
                if in_entry {
                    out.push(Event::Html("</article>".into()));
                }
                out.push(Event::Html(
                    r#"<article class="entry"><header class="entry-head">"#.into(),
                ));
                out.push(Event::Start(Tag::Heading(HeadingLevel::H3, id, classes)));
                in_entry = true;
                in_entry_head = true;
            }
            Event::Start(Tag::Heading(HeadingLevel::H4, id, classes)) => {
                in_h4 = true;
                out.push(Event::Start(Tag::Heading(HeadingLevel::H4, id, classes)));
            }
            Event::End(Tag::Heading(HeadingLevel::H4, id, classes)) => {
                in_h4 = false;
                out.push(Event::End(Tag::Heading(HeadingLevel::H4, id, classes)));
                if in_entry_head {
                    out.push(Event::Html("</header>".into()));
                    in_entry_head = false;
                }
            }
            Event::Text(text) if in_h4 => {
                let rendered = text
                    .replace(
                        "NYC",
                        r#"<span class="city-chip"><img src="./static/nyc.png" alt=""/>NYC</span>"#,
                    )
                    .replace(
                        "London",
                        r#"<span class="city-chip"><img src="./static/london.png" alt=""/>London</span>"#,
                    )
                    .replace(
                        "Berlin",
                        r#"<span class="city-chip"><img src="./static/berlin.png" alt=""/>Berlin</span>"#,
                    );
                out.push(Event::Html(rendered.into()));
            }
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

    if in_entry_head {
        out.push(Event::Html("</header>".into()));
    }
    if in_entry {
        out.push(Event::Html("</article>".into()));
    }
    if in_section {
        out.push(Event::Html("</div></section>".into()));
    }
    if in_intro {
        if is_readme_file {
            out.push(Event::Html("</div>".into()));
        } else {
            out.push(Event::Html("</div></div>".into()));
        }
    }

    Ok(out)
}
