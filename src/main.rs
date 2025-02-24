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
    // Setup output directory
    let _ = fs::remove_dir_all(OUTPUT_DIR);
    fs::create_dir(OUTPUT_DIR).expect("failed to create output dir");

    // Copy static assets to output directory
    for entry in WalkDir::new(STATIC_DIR) {
        let path = entry.expect("failed to get dir entry").into_path();
        let file_type = path.metadata().expect("failed to get metadata").file_type();
        let target_path = PathBuf::from(OUTPUT_DIR).join(&path);

        if file_type.is_file() {
            fs::copy(&path, target_path).expect("failed copy file");
        } else if file_type.is_dir() {
            fs::create_dir(target_path).expect("failed to copy subdir");
        }
    }

    // Find all markdown files to process
    let markdown_files = HashSet::<PathBuf>::from_iter(
        WalkDir::new("./")
            .into_iter()
            .filter_entry(|entry| {
                let path = entry.path();
                !IGNORED_MD_DIRECTORIES.iter().any(|dir| path.starts_with(dir))
            })
            .filter_map(|entry_result| {
                let path = entry_result.expect("failed to get dir entry").into_path();
                if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                    Some(path)
                } else {
                    None
                }
            }),
    );

    // Process each markdown file into HTML
    for markdown_path in markdown_files.iter() {
        let markdown_content = fs::read_to_string(markdown_path).expect("failed to read markdown");

        // Transform markdown to HTML with special link handling
        let transformed_parser = Parser::new(&markdown_content).map(|event| match event {
            Event::Start(Tag::Link(link_type, mut destination, title)) => {
                if destination.ends_with(".md") {
                    let dest_str = destination.to_string();
                    let destination_path = Path::new(&dest_str);
                    let full_dest_path = markdown_path.parent().unwrap_or(Path::new("./")).join(destination_path);

                    if markdown_files.contains(&full_dest_path) {
                        destination = make_html_path(&PathBuf::from(destination_path))
                            .to_string_lossy()
                            .into_owned()
                            .into();
                    }
                }
                Event::Start(Tag::Link(link_type, destination.clone(), title.clone()))
            },
            Event::End(Tag::Link(link_type, destination, title)) => {
                let is_external_link = destination.starts_with("http://") || destination.starts_with("https://");
                if is_external_link {
                    // Add external link icon
                    Event::Html(
                        r#"<svg style="width: 0.4em; vertical-align: middle; padding-bottom: 0.4em;" class="w-16 align-top" focusable="false" aria-hidden="true" viewBox="3 6 23 20"><path stroke="currentcolor" stroke-width="4" fill="none" d="M24 8L8 24M8 8H24v16"></path></svg></a>"#.into()
                    )
                } else {
                    Event::End(Tag::Link(link_type, destination, title))
                }
            },
            _ => event,
        });

        // Generate HTML file path and ensure parent directories exist
        let html_output_path = make_html_path(markdown_path);
        let target_html_path = PathBuf::from(OUTPUT_DIR).join(&html_output_path);

        if let Some(parent_dir) = target_html_path.parent() {
            fs::create_dir_all(parent_dir).expect("Failed to create html parent dirs");
        }

        // Convert markdown to HTML and apply transformations
        let mut html_content = String::new();
        html::push_html(&mut html_content, transformed_parser);

        // Replace newlines with spaces and colorize Google mentions
        let formatted_html = html_content
            .chars()
            .map(|c| if c == '\n' { '\u{0020}' } else { c })
            .collect::<String>()
            .replace("Google", r#"<span style="color: var(--gblue)">G</span><span style="color: var(--gred)">o</span><span style="color: var(--gyellow)">o</span><span style="color: var(--gblue)">g</span><span style="color: var(--ggreen)">l</span><span style="color: var(--gred)">e</span>"#);

        // Write the final HTML file
        fs::write(target_html_path, html_page(&html_output_path, formatted_html))
            .expect("Failed to write html");
    }
}

fn make_html_path(md_path: &Path) -> PathBuf {
    let mut html_path = md_path.to_path_buf();
    let is_readme = html_path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_lowercase())
        == Some("readme.md".to_string());

    if is_readme {
        html_path.set_file_name("index");
    }
    html_path.set_extension("html");
    html_path
}

fn html_page(html_path: &Path, html_fragment: String) -> String {
    let file_stem = html_path
        .file_stem()
        .and_then(|s| s.to_str())
        .expect("md file_name");

    let page_title = if file_stem == "index" {
        "Juliette Pluto".to_string()
    } else {
        format!("{} — Juliette Pluto", file_stem)
    };

    format!(
        r##"
<!DOCTYPE html>
<html lang="en">
    <head>
        <meta charset="utf-8" />
        <meta name="viewport" content="width=device-width,initial-scale=1,shrink-to-fit=no" />
        <title>{page_title}</title>
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
    )
}
