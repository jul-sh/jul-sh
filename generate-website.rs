use pulldown_cmark::{html, Event, Parser, Tag};
use relative_path::RelativePathBuf;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

static OUTPUT_DIR: &str = "./build";

fn main() {
    let _ = fs::remove_dir_all(OUTPUT_DIR);
    fs::create_dir(OUTPUT_DIR).expect("failed to create output dir");

    // Copy static files
    for entry in WalkDir::new("./static") {
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
    let md_paths = HashSet::<_>::from_iter(
        WalkDir::new("./")
            .into_iter()
            .filter_entry(|e| {
                !e.path().starts_with("./static")
                    && !e.path().starts_with("./target")
                    && !e.path().starts_with("./.git")
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
                let mut html_buf = String::new();
                html::push_html(&mut html_buf, parsed);
                html_buf
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
    let body = format!(r##"<div id="content-wrapper">{}</div>"##, html_fragment);
    return format!(
        r##"
<!DOCTYPE html>
<html lang="en">
    <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width,initial-scale=1,shrink-to-fit=no" />
    <title>{}</title>
    <meta name="description" content="Engineer at Google" />
    <link rel="stylesheet" href="./static/main.css">
    <link rel="apple-touch-icon" sizes="180x180" href="./static/apple-touch-icon.png" />
    <link rel="icon" type="image/png" sizes="32x32" href="./static/favicon-32x32.png" />
    <meta name="theme-color" content="#101723" />
    </head>
    <body>
    {}
    </body>
</html>
"##,
        title, body
    );
}
