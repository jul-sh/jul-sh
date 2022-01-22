/*
A simple deno script that collects the markdown files in the root directory and
generates static HTML pages off them. For more about deno see https://deno.land.
Run via `deno run --allow-read="./" --allow-write="./build" build.ts`.
*/

import { Marked, Renderer } from "https://deno.land/x/markdown@v2.0.0/mod.ts";
import * as path from "https://deno.land/std@0.122.0/path/mod.ts";
import * as fs from "https://deno.land/std@0.122.0/fs/mod.ts";

const BUILD_DIR = "./build";
await fs.emptyDir(BUILD_DIR);
for await (const entry of fs.walk("./static", { includeDirs: false })) {
  const target = `${BUILD_DIR}/${entry.path}`;
  await fs.ensureFile(target);
  await Deno.copyFile(
    entry.path,
    target,
  );
}

const MarkdownPaths = new Set<string>();
for await (const { isFile, name: path } of Deno.readDir("./")) {
  if (isFile && path.endsWith(".md")) {
    MarkdownPaths.add(path);
  }
}

class MarkdownRenderer extends Renderer {
  static htmlPath(markdownPath: string): string {
    const path = markdownPath.toLocaleLowerCase().slice(0, -2) + "html";
    return path === "readme.html" ? "index.html" : path;
  }
  link(
    ...[href, ...rest]: Parameters<Renderer["link"]>
  ): ReturnType<Renderer["link"]> {
    const normalizedPath = path.normalize(href);
    if (MarkdownPaths.has(normalizedPath)) {
      return super.link(MarkdownRenderer.htmlPath(normalizedPath), ...rest);
    } else {
      return super.link(href, ...rest);
    }
  }
}
Marked.setOptions({ renderer: new MarkdownRenderer() });

for (const markdownPath of MarkdownPaths.values()) {
  const markdown = await Deno.readTextFile(markdownPath);
  const htmlFragment = Marked.parse(markdown).content;
  const path = MarkdownRenderer.htmlPath(markdownPath);

  await Deno.writeTextFile(
    `${BUILD_DIR}/${path}`,
    `<!DOCTYPE html>
  <html lang="en">
    <head>
      <meta charset="utf-8" />
      <meta name="viewport" content="width=device-width,initial-scale=1,shrink-to-fit=no" />
      <title>${
      path === "index.html" ? "" : `${path.slice(0, -5)} — `
    }Juliette Pretot</title>
      <meta name="description" content="Engineer at Google" />
      <link rel="stylesheet" href="./static/main.css">
      <link rel="apple-touch-icon" sizes="180x180" href="./static/apple-touch-icon.png" />
      <link rel="icon" type="image/png" sizes="32x32" href="./static/favicon-32x32.png" />
      <meta name="theme-color" content="#101723" />
    </head>
    <body>
    ${
      path === "index.html"
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
    { create: true },
  );
}
