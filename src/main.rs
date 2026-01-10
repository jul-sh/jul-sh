//! Minimal static-site generator: convert *.md → HTML, mirror ./static.

use eyre::Result;
use pulldown_cmark::{Event, Parser, Tag, html};
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
// Whether and what image of me on index.html
const INDEX_IMAGE: IndexImage = IndexImage::None;

#[derive(Clone, Copy)]
enum IndexImage {
    None,
    Photo(&'static str),
}

enum PageKind {
    Index { title: String, image: IndexImage },
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
    let fragment_ref = fragment.as_str();
    let (title, main) = match PageKind::try_from(html_rel_path)? {
        PageKind::Index { title, image } => {
            let main = match image {
                IndexImage::None => format!(r#"<main>{fragment_ref}</main>"#),
                IndexImage::Photo(image_file) => format!(
                    r#"<main class="wide">
      <div class="index-photo"><img src="./static/{}" alt="Photo of Juliette Pluto"></div>
      <div class="index-content">{}</div>
    </main>"#,
                    image_file, fragment_ref
                ),
            };
            (title, main)
        }
        PageKind::Regular { title } => (title, format!(r#"<main>{fragment_ref}</main>"#)),
    };
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width,initial-scale=1,shrink-to-fit=no"/>
    <title>{title}</title>
    <meta name="description" content="Engineer at Google"/>
    <link rel="stylesheet" href="./static/main.css"/>
    <link rel="preload" href="./static/iosevka-julsh-curly-regular.woff2" as="font" type="font/woff2"/>
    <link rel="preload" href="./static/iosevka-julsh-curly-bold.woff2" as="font" type="font/woff2"/>
    <link rel="apple-touch-icon" sizes="180x180" href="./static/apple-touch-icon.png"/>
    <link rel="icon" type="image/png" sizes="32x32" href="./static/favicon-32x32.png"/>
    <meta name="theme-color" content="#11161d"/>
  </head>
  <body>
    {main}
  </body>
  <!-- 🗽 -->
</html>
"##
    )
    .into()
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
                image: INDEX_IMAGE,
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

    const GOOGLE_HTML: &str = r#"<span style="color: var(--gblue)">G</span><span style="color: var(--gred)">o</span><span style="color: var(--gyellow)">o</span><span style="color: var(--gblue)">g</span><span style="color: var(--ggreen)">l</span><span style="color: var(--gred)">e</span>"#;

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
            // If the link is external (http/https), append an inline SVG icon *before* the end tag.
            Event::End(Tag::Link(link_ty, dest, title)) => {
                out.push(Event::End(Tag::Link(link_ty, dest, title)));
            }
            // In text on readme, colorize Google.
            Event::Text(text) if is_readme_file && text.contains("Google") => {
                let s = text.into_string();
                out.extend(s.split("Google").enumerate().flat_map(|(i, part)| {
                    let html = (i > 0).then(|| Event::Html(GOOGLE_HTML.into()));
                    let txt = (!part.is_empty()).then(|| Event::Text(part.to_string().into()));
                    html.into_iter().chain(txt)
                }));
            }
            other => out.push(other),
        }
    }
    Ok(out)
}
