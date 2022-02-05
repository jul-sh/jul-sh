use pulldown_cmark;
use relative_path::RelativePathBuf;
use std::fs;
use walkdir;

static OUTPUT_DIR: &str = "./build";

fn main() -> std::io::Result<()> {
    fs::remove_dir_all(OUTPUT_DIR).expect("failed to clear output dir");
    fs::create_dir(OUTPUT_DIR).expect("failed to create output dir");

    copy_recursively(
        std::path::PathBuf::from("./static"),
        std::path::PathBuf::from(OUTPUT_DIR),
    )
    .expect("failed to copy static assets");

    let md_paths = std::collections::HashSet::<_>::from_iter(
        walkdir::WalkDir::new("./").into_iter().filter_map(|e| {
            let path = RelativePathBuf::from_path(
                &e.expect("failed to turn md walker entry into path")
                    .into_path(),
            )
            .expect("failed to create relative markdown path");
            if !path.starts_with("./target")
                && !path.starts_with("./.git")
                && path.extension() == Some("md")
            {
                Some(path.normalize())
            } else {
                None
            }
        }),
    );

    for md_path in md_paths.iter() {
        let contents = fs::read_to_string(&md_path.to_path(".")).expect("failed to read markdown");
        let parsed = pulldown_cmark::Parser::new(&contents).map(|event| match event {
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link(
                link_type,
                mut destination,
                title,
            )) => {
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
                pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link(
                    link_type,
                    destination,
                    title,
                ))
            }
            _ => event,
        });

        let html_path = make_html_path(md_path.clone());
        let html = html(&html_path, {
            let mut html_buf = String::new();
            pulldown_cmark::html::push_html(&mut html_buf, parsed);
            html_buf
        });

        fs::write(
            {
                let target = html_path.to_path(OUTPUT_DIR);
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).expect("Failed to create html parent dirs");
                };
                target
            },
            html,
        )
        .expect("Failed to write html");
    }

    Ok(())
}

fn copy_recursively(src: std::path::PathBuf, dst: std::path::PathBuf) -> std::io::Result<()> {
    Ok(())
}

fn make_html_path(mut md_path: RelativePathBuf) -> RelativePathBuf {
    if md_path.file_name().expect("html file_name").to_lowercase() == "readme.md" {
        md_path.set_file_name("index");
    }
    md_path.set_extension("html");
    md_path
}

fn html(html_path: &RelativePathBuf, html_fragment: String) -> String {
    let file_name = html_path.file_name().expect("md file_name");
    let title = if file_name == "index" {
        "Juliette Pretot".to_string()
    } else {
        format!("{} — Juliette Pretot", file_name)
    };
    let body = if file_name == "index" {
        let picture = r##"<picture>
<source type="image/webp" srcset="./static/me-4by5.webp"></source>
<source type="image/jpeg" srcset="./static/me-4by5.jpg"></source>
<img src="./static/me-4by5.jpg" alt="Juliette in front of the Golden Gate bridge" width="100%"/>
</picture>"##;
        format!(
            r##"<div id="content-wrapper" class="page-index">{}<main>{}</main></div>"##,
            picture, html_fragment
        )
    } else {
        format!(r##"<div id="content-wrapper">{}</div>"##, html_fragment)
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
