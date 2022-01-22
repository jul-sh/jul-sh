/*
A simple deno script that collects the markdown files in the root directory and
generates static HTML pages off them. For more about deno see https://deno.land.
Run via `deno run --allow-read="./" --allow-write="./build" build.ts`.
*/

import { Marked, Renderer } from "https://deno.land/x/markdown@v2.0.0/mod.ts";
import { normalize } from "https://deno.land/std@0.122.0/path/mod.ts";

/*
Deno Helper Functions
*/

// Recursively copies a directory. Should be replaced with `fs.copy` from the
// deno std library once that is stable.
async function copyDir(source: string, destination: string) {
  for await (const dirEntry of Deno.readDir(source)) {
    if (dirEntry.isDirectory) {
      Deno.mkdir(`${destination}/${dirEntry.name}`);
      await copyDir(
        `${source}/${dirEntry.name}`,
        `${destination}/${dirEntry.name}`,
      );
    } else {
      await Deno.copyFile(
        `${source}/${dirEntry.name}`,
        `${destination}/${dirEntry.name}`,
      );
    }
  }
}

// Creates an empty directory, replacing any existing ones. Should be replaced
// with `fs.emptyDir` from the deno std library once that is stable.
async function emptyDir(path: string) {
  try {
    await Deno.remove(path, { recursive: true });
  } catch (error) {
    if (error instanceof Deno.errors.NotFound) {
      // nothing to remove
    } else {
      throw error;
    }
  }

  await Deno.mkdir(path);
}

/*
Main Script
*/

function createHTMLPage(
  htmlName: string,
  htmlSegment: string,
  css: string,
): string {
  return `<!DOCTYPE html>
  <html lang="en">
    <head>
      <meta charset="utf-8" />
      <meta name="viewport" content="width=device-width,initial-scale=1,shrink-to-fit=no" />
      <title>${
    htmlName === "index" ? "" : `${htmlName} — `
  }Juliette Pretot</title>
      <meta name="description" content="Engineer at Google" />
      <style>${css}</style>
      <link rel="apple-touch-icon" sizes="180x180" href="./static/apple-touch-icon.png" />
      <link rel="icon" type="image/png" sizes="32x32" href="./static/favicon-32x32.png" />
      <meta name="theme-color" content="#101723" />
    </head>
    <body>
      <div id="content-wrapper" class="page-${htmlName}">${
    htmlName === "index"
      ? `<picture>
          <div class="image-placeholder"></div>
          <source type="image/webp" srcset="./static/me-4by5.webp">
          <source type="image/jpeg" srcset="./static/me-4by5.jpg">
          <img src="./static/me-4by5.jpg" alt="Juliette in front of the Golden Gate bridge" width="100%"></img>
        </picture>
        <main>${htmlSegment}</main>`
      : htmlSegment
  }</div>
    </body>
  </html>`;
}

function markdownPathToHtmlName(path: string): string {
  const extensionLessPath = path.substr(0, path.length - 3).toLocaleLowerCase();
  return extensionLessPath === "readme" ? "index" : extensionLessPath;
}

const BUILD_DIR = "./build";
const CSS = await Deno.readTextFile("./main.css");

await emptyDir(BUILD_DIR);
await Deno.mkdir(`${BUILD_DIR}/static`);
await copyDir("./static", `${BUILD_DIR}/static`);

const LocalMarkdownFiles = new Set<string>();
for await (const { isFile, name } of Deno.readDir("./")) {
  if (isFile && name.endsWith(".md")) {
    LocalMarkdownFiles.add(name);
  }
}

class JuliettesMarkdownRenderer extends Renderer implements Renderer {
  link(
    ...[href, ...rest]: Parameters<Renderer["link"]>
  ): ReturnType<Renderer["link"]> {
    const normalizedPath = normalize(href);
    if (LocalMarkdownFiles.has(normalizedPath)) {
      const htmlHref = `${markdownPathToHtmlName(normalizedPath)}.html`;
      return super.link(htmlHref, ...rest);
    } else {
      return super.link(href, ...rest);
    }
  }
}
Marked.setOptions({ renderer: new JuliettesMarkdownRenderer() });

for (const markdownFilePath of LocalMarkdownFiles.values()) {
  const markdown = await Deno.readTextFile(markdownFilePath);
  const htmlSegment = Marked.parse(markdown)
    .content;
  const htmlName = markdownPathToHtmlName(markdownFilePath);
  const htmlPage = createHTMLPage(htmlName, htmlSegment, CSS);
  await Deno.writeTextFile(
    `${BUILD_DIR}/${htmlName}.html`,
    htmlPage,
    { create: true },
  );
}
