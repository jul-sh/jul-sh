//! A simple static site generator that recursively finds markdown files in the current
//! directory, and generates HTML documents based on them.

// Output the generated static site here.
static OUTPUT_DIR: &str = "./build";
// Copy this static directory into output. Used for font files, images, etc.
static STATIC_DIR: &str = "./static";
// Don't look for markdown files in these directories.
static IGNORED_MD_DIRECTORIES: &[&str] = &["./target", "./.git", STATIC_DIR, OUTPUT_DIR];

use pulldown_cmark::{html, Event, Parser, Tag};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn main() {
    let _ = fs::remove_dir_all(OUTPUT_DIR);
    fs::create_dir(OUTPUT_DIR).expect("failed to create output dir");

    // Copy static files
    for entry in WalkDir::new(STATIC_DIR) {
        let path = entry.expect("failed to get dir entry").into_path();
        let file_type = path.metadata().expect("failed to get metadata").file_type();
        let target = PathBuf::from(OUTPUT_DIR).join(&path);
        if file_type.is_file() {
            fs::copy(&path, target).expect("failed copy file");
        } else if file_type.is_dir() {
            fs::create_dir(target).expect("failed to copy subdir");
        }
    }

    // Get markdown files
    let md_paths = HashSet::<PathBuf>::from_iter(
        WalkDir::new("./")
            .into_iter()
            .filter_entry(|e| {
                let is_in_ignored_dir = !IGNORED_MD_DIRECTORIES
                    .into_iter()
                    .any(|dir| e.path().starts_with(dir));
                is_in_ignored_dir
            })
            .filter_map(|e| {
                let path = e.expect("failed to get dir entry").into_path();
                if path.extension().and_then(|e| e.to_str()) == Some("md") {
                    Some(path)
                } else {
                    None
                }
            }),
    );

    // Generate HTML files off them
    for md_path in md_paths.iter() {
        let contents = fs::read_to_string(&md_path).expect("failed to read markdown");
        let parsed = Parser::new(&contents).map(|event| match event {
            Event::Start(Tag::Link(link_type, mut destination, title)) => {
                if destination.ends_with(".md") {
                    let dest_str = destination.to_string();
                    let destination_path = Path::new(&dest_str);
                    let full_dest_path = md_path.parent().unwrap_or(Path::new("./")).join(destination_path);
                    if md_paths.contains(&full_dest_path) {
                        destination = make_html_path(&PathBuf::from(destination_path))
                            .to_string_lossy()
                            .into_owned()
                            .into();
                    }
                }
                Event::Start(Tag::Link(link_type, destination, title))
            }
            _ => event,
        });

        let html_path = make_html_path(&md_path);
        fs::write(
            {
                let target = PathBuf::from(OUTPUT_DIR).join(&html_path);
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).expect("Failed to create html parent dirs");
                };
                target
            },
            html_page(&html_path, {
                let mut html = String::new();
                html::push_html(&mut html, parsed);
                let html: String = html
                    .chars()
                    .into_iter()
                    .map(|c| if c == '\n' { '\u{0020}' } else { c })
                    .collect();
                // Render Google in nice colors
                html.replace("Google", r#"<span style="color: var(--gblue)">G</span><span style="color: var(--gred)">o</span><span style="color: var(--gyellow)">o</span><span style="color: var(--gblue)">g</span><span style="color: var(--ggreen)">l</span><span style="color: var(--gred)">e</span>"#)
            }),
        )
        .expect("Failed to write html");
    }
}

fn make_html_path(md_path: &Path) -> PathBuf {
    let mut html_path = md_path.to_path_buf();
    if html_path.file_name().and_then(|n| n.to_str()).map(|s| s.to_lowercase()) == Some("readme.md".to_string()) {
        html_path.set_file_name("index");
    }
    html_path.set_extension("html");
    html_path
}

fn html_page(html_path: &Path, html_fragment: String) -> String {
    let file_name = html_path.file_stem().and_then(|s| s.to_str()).expect("md file_name");
    let title = if file_name == "index" {
        "Juliette Pluto".to_string()
    } else {
        format!("{} — Juliette Pluto", file_name)
    };
    return format!(
        r##"
<!DOCTYPE html>
<html lang="en">
    <head>
        <meta charset="utf-8" />
        <meta name="viewport" content="width=device-width,initial-scale=1,shrink-to-fit=no" />
        <title>{title}</title>
        <meta name="description" content="Engineer at Google" />
        <link rel="stylesheet" href="./static/main.css" />
        <link rel="preload" href="./static/iosevka-julsh-curly-regular.woff2" as="font" type="font/woff2" />
        <link rel="preload" href="./static/iosevka-julsh-curly-bold.woff2" as="font" type="font/woff2" />
        <link rel="apple-touch-icon" sizes="180x180" href="./static/apple-touch-icon.png" />
        <link rel="icon" type="image/png" sizes="32x32" href="./static/favicon-32x32.png" />
        <meta name="theme-color" content="#11161d" />
    </head>
    <body>
        <main>{html_fragment}</main>
    </body>
    <!-- 🗽 -->
</html>
"##
    );
}
