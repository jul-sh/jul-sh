//! A simple static site generator that recursively finds markdown files in the current
//! directory, and generates HTML documents based on them.

// Output the generated static site here.
static OUTPUT_DIR: &str = "./build";
// Copy this static directory into output. Used for font files, images, etc.
static STATIC_DIR: &str = "./static";
// Don't look for markdown files in these directories.
static IGNORED_MD_DIRECTORIES: &[&str] = &["./target", "./.git", STATIC_DIR, OUTPUT_DIR];

use pulldown_cmark::{html, Event, Parser, Tag};
use relative_path::RelativePathBuf;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
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
    let md_paths = HashSet::<RelativePathBuf>::from_iter(
        WalkDir::new("./")
            .into_iter()
            .filter_entry(|e| {
                let is_in_ignored_dir = !IGNORED_MD_DIRECTORIES
                    .into_iter()
                    .any(|dir| e.path().starts_with(dir));
                is_in_ignored_dir
            })
            .filter_map(|e| {
                let path = RelativePathBuf::from_path(
                    &e.expect("failed to turn md walker entry into path")
                        .into_path(),
                )
                .expect("failed to create relative markdown path");
                if path.extension() == Some("md") {
                    Some(path.normalize())
                } else {
                    None
                }
            }),
    );

    // Generate HTML files off them
    for md_path in md_paths.iter() {
        let contents = fs::read_to_string(&md_path.to_path(".")).expect("failed to read markdown");
        let parsed = Parser::new(&contents).map(|event| match event {
            Event::Start(Tag::Link(link_type, mut destination, title)) => {
                if destination.ends_with(".md") {
                    let destination_path = RelativePathBuf::from(destination.to_string());
                    if md_paths.contains(
                        &md_path
                            .parent()
                            .unwrap_or(&RelativePathBuf::from("./"))
                            .join_normalized(&destination_path),
                    ) {
                        destination = make_html_path(destination_path).as_str().to_string().into()
                    }
                }
                Event::Start(Tag::Link(link_type, destination, title))
            }
            _ => event,
        });

        let html_path = make_html_path(md_path.clone());
        fs::write(
            {
                let target = html_path.to_path(OUTPUT_DIR);
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
                html
            }),
        )
        .expect("Failed to write html");
    }
}

fn make_html_path(mut md_path: RelativePathBuf) -> RelativePathBuf {
    if md_path.file_name().expect("html file_name").to_lowercase() == "readme.md" {
        md_path.set_file_name("index");
    }
    md_path.set_extension("html");
    md_path
}

fn html_page(html_path: &RelativePathBuf, html_fragment: String) -> String {
    let file_name = html_path.file_stem().expect("md file_name");
    let title = if file_name == "index" {
        "Juliette Pretot".to_string()
    } else {
        format!("{} — Juliette Pretot", file_name)
    };
    return format!(
        r##"
<!DOCTYPE html>
<html lang="en">
    <head>
        <meta charset="utf-8" />
        <meta name="viewport" content="width=device-width,initial-scale=1,shrink-to-fit=no" />
        <title>{}</title>
        <meta name="description" content="Engineer at Google" />
        <link rel="stylesheet" href="./static/main.css" />
        <link rel="preload" href="./static/iosevka-julsh-curly-regular.woff2" as="font" type="font/woff2" />
        <link rel="preload" href="./static/iosevka-julsh-curly-bold.woff2" as="font" type="font/woff2" />
        <link rel="apple-touch-icon" sizes="180x180" href="./static/apple-touch-icon.png" />
        <link rel="icon" type="image/png" sizes="32x32" href="./static/favicon-32x32.png" />
        <meta name="theme-color" content="#11161d" />
    </head>
    <body>
        <main>{}</main>
    </body>
</html>
"##,
        title, html_fragment
    );
}
