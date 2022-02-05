use pulldown_cmark;
use relative_path::{RelativePath, RelativePathBuf};
use std::fs;
use walkdir;

static OUTPUT_DIR: &str = "./build";

fn make_html_path(mut markdown_path: RelativePathBuf) -> RelativePathBuf {
    if markdown_path.file_name().unwrap().to_lowercase() == "readme.md" {
        markdown_path.set_file_name("index");
    }
    markdown_path.set_extension("html");
    markdown_path
}

fn main() -> std::io::Result<()> {
    let _ = fs::remove_dir_all(OUTPUT_DIR);
    fs::create_dir(OUTPUT_DIR)?;

    for entry in walkdir::WalkDir::new("./static") {
        let path = entry?.into_path();
        if path.is_file() {
            let target = std::path::Path::new(OUTPUT_DIR).join(&path);
            if target.parent().is_some() {
                fs::create_dir_all(target.parent().unwrap()).unwrap();
            }
            fs::copy(&path, target)?;
        }
    }

    let walker = walkdir::WalkDir::new("./").into_iter();
    let set = std::collections::HashSet::<_>::from_iter(walker.filter_map(|e| {
        let p = &e.ok()?.into_path();
        let path = RelativePathBuf::from_path(p).ok()?;
        if !path.starts_with("./target") && !path.starts_with("./.git") && path.extension()? == "md"
        {
            Some(path.normalize())
        } else {
            None
        }
    }));

    for markdown_path in set.iter() {
        let contents = fs::read_to_string(&markdown_path.to_path("."))?;
        let events = pulldown_cmark::Parser::new_ext(&contents, pulldown_cmark::Options::empty())
            .map(|event| match event {
                pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link(
                    link_type,
                    destination,
                    title,
                )) => {
                    let mut href = RelativePathBuf::from(destination.to_string());
                    if href.extension().is_some() && href.extension().unwrap() == "md" {
                        let joined = markdown_path
                            .parent()
                            .unwrap_or(&RelativePath::new(OUTPUT_DIR))
                            .join_normalized(&href);
                        if set.contains(&joined) {
                            href = make_html_path(href);
                        }
                    }

                    pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link(
                        link_type,
                        href.as_str().to_string().into(),
                        title,
                    ))
                }
                _ => event,
            })
            .into_iter();

        let html_path = make_html_path(markdown_path.clone());

        let html_fragment = {
            let mut html_buf = String::new();
            pulldown_cmark::html::push_html(&mut html_buf, events);
            html_buf
        };

        let html = html(&html_path, html_fragment);

        let target = html_path.to_path(OUTPUT_DIR);
        if target.parent().is_some() {
            fs::create_dir_all(target.parent().unwrap()).unwrap();
        }
        fs::write(target, html)?;
    }

    Ok(())
}

fn html(html_path: &RelativePathBuf, html_fragment: String) -> String {
    let file_name = html_path.file_name().unwrap();
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
