// Generates static HTML pages off the markdown files in the repo. Rewrites
// links between them as links between the generated HTML pages.
// Run via `deno run --allow-read="./" --allow-write="./build" build.ts`.

import * as markdown from "https://deno.land/x/markdown@v2.0.0/mod.ts";
import * as path from "https://deno.land/std@0.122.0/path/mod.ts";
import * as fs from "https://deno.land/std@0.122.0/fs/mod.ts";

const BUILD_DIR = "./build";
await fs.emptyDir(BUILD_DIR);
for await (const entry of fs.walk("./static", { includeDirs: false })) {
  const target = path.join(BUILD_DIR, entry.path);
  await fs.ensureFile(target);
  await Deno.copyFile(entry.path, target);
}

const MarkdownPaths = new Set<string>();
for await (
  const entry of fs.walk("./", {
    includeDirs: false,
    exts: ["md"],
    skip: [new RegExp("static")],
  })
) {
  MarkdownPaths.add(entry.path);
}

class MarkdownRenderer extends markdown.Renderer {
  static htmlPath(markdownPath: string): string {
    const path = markdownPath.toLocaleLowerCase().slice(0, -2) + "html";
    return path === "readme.html" ? "index.html" : path;
  }
  #dir: string;
  constructor(dir: string) {
    super();
    this.#dir = dir;
  }
  link(
    ...[href, ...rest]: Parameters<markdown.Renderer["link"]>
  ): ReturnType<markdown.Renderer["link"]> {
    if (MarkdownPaths.has(path.join(this.#dir, href))) {
      return super.link(MarkdownRenderer.htmlPath(href), ...rest);
    } else return super.link(href, ...rest);
  }
}

for (const markdownPath of MarkdownPaths.values()) {
  const htmlFragment =
    markdown.Marked.parse(await Deno.readTextFile(markdownPath), {
      ...new markdown.MarkedOptions(),
      renderer: new MarkdownRenderer(path.dirname(markdownPath)),
    }).content;
  const htmlPath = MarkdownRenderer.htmlPath(markdownPath);

  await fs.ensureFile(path.join(BUILD_DIR, htmlPath));
  await Deno.writeTextFile(
    path.join(BUILD_DIR, htmlPath),
    `<!DOCTYPE html>
  <html lang="en">
    <head>
      <meta charset="utf-8" />
      <meta name="viewport" content="width=device-width,initial-scale=1,shrink-to-fit=no" />
      <title>${
      htmlPath === "index.html" ? "" : `${htmlPath.slice(0, -5)} — `
    }Juliette Pretot</title>
      <meta name="description" content="Engineer at Google" />
      <link rel="stylesheet" href="./static/main.css">
      <link rel="apple-touch-icon" sizes="180x180" href="./static/apple-touch-icon.png" />
      <link rel="icon" type="image/png" sizes="32x32" href="./static/favicon-32x32.png" />
      <meta name="theme-color" content="#101723" />
    </head>
    <body>
    ${
      htmlPath === "index.html"
        ? `<div id="content-wrapper" class="page-index"><picture>
        <div class="image-placeholder"></div>
        <source type="image/webp" srcset="./static/me-4by5.webp">
        <source type="image/jpeg" srcset="./static/me-4by5.jpg">
        <img src="./static/me-4by5.jpg" alt="Juliette in front of the Golden Gate bridge" width="100%"></img>
      </picture>
      <main>${htmlFragment}</main></div>`
        : `<div id="content-wrapper">${htmlFragment}</div>`
    }</div>
    </body>
  </html>`,
  );
}
