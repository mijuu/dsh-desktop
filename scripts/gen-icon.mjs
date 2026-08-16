import { Resvg } from "@resvg/resvg-js";
import { writeFileSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const SIZE = 1024;

// macOS icon spec (1024x1024 canvas):
//   Content area: 824x824 px, centered
//   Padding: 100px on each side
//   Corner radius: 185px (superellipse / squircle)
const MAC_CONTENT = 824;
const MAC_PADDING = (SIZE - MAC_CONTENT) / 2; // 100
const MAC_RADIUS = 185;

// Windows icon spec (1024x1024 canvas):
//   Full canvas with rounded corners
//   Corner radius: ~10% of size = 100px
const WIN_RADIUS = 100;

function main() {
  const svgPath = join(__dirname, "../src-tauri/app-icon-source.svg");
  let svg = readFileSync(svgPath, "utf-8");

  // Extract the inner content of the SVG
  const innerContent = svg.replace(/<svg[^>]*>/, "").replace("</svg>", "");

  // Original SVG viewBox: 0 0 23.16 17.04
  // Scale to fit within the content area with some inner padding
  const innerPad = MAC_CONTENT * 0.15; // 15% inner padding
  const iconArea = MAC_CONTENT - innerPad * 2;
  const scaleX = iconArea / 23.16;
  const scaleY = iconArea / 17.04;
  const scale = Math.min(scaleX, scaleY);
  const iconW = 23.16 * scale;
  const iconH = 17.04 * scale;
  const offsetX = (SIZE - iconW) / 2;
  const offsetY = (SIZE - iconH) / 2;

  // === macOS: squircle (rounded rect) 824x824, 100px padding, 185px radius ===
  const macSvg = `<svg width="${SIZE}" height="${SIZE}" viewBox="0 0 ${SIZE} ${SIZE}" xmlns="http://www.w3.org/2000/svg">
    <defs>
      <clipPath id="mac-clip">
        <rect x="${MAC_PADDING}" y="${MAC_PADDING}" width="${MAC_CONTENT}" height="${MAC_CONTENT}" rx="${MAC_RADIUS}" ry="${MAC_RADIUS}"/>
      </clipPath>
    </defs>
    <g clip-path="url(#mac-clip)">
      <rect x="${MAC_PADDING}" y="${MAC_PADDING}" width="${MAC_CONTENT}" height="${MAC_CONTENT}" rx="${MAC_RADIUS}" ry="${MAC_RADIUS}" fill="black"/>
      <g transform="translate(${offsetX}, ${offsetY}) scale(${scale})">
        ${innerContent}
      </g>
    </g>
  </svg>`;

  const resvgMac = new Resvg(macSvg, { background: "transparent", fitTo: { mode: "original" } });
  const macPng = resvgMac.render().asPng();
  writeFileSync(join(__dirname, "../src-tauri/app-icon.png"), macPng);
  console.log("wrote app-icon.png (macOS, 824x824 squircle, r=185):", macPng.length, "bytes");

  // === Windows: full canvas with rounded corners (r=100), transparent outside ===
  const winSvg = `<svg width="${SIZE}" height="${SIZE}" viewBox="0 0 ${SIZE} ${SIZE}" xmlns="http://www.w3.org/2000/svg">
    <defs>
      <clipPath id="win-clip">
        <rect width="${SIZE}" height="${SIZE}" rx="${WIN_RADIUS}" ry="${WIN_RADIUS}"/>
      </clipPath>
    </defs>
    <g clip-path="url(#win-clip)">
      <rect width="${SIZE}" height="${SIZE}" rx="${WIN_RADIUS}" ry="${WIN_RADIUS}" fill="black"/>
      <g transform="translate(${offsetX}, ${offsetY}) scale(${scale})">
        ${innerContent}
      </g>
    </g>
  </svg>`;

  const resvgWin = new Resvg(winSvg, { background: "transparent", fitTo: { mode: "original" } });
  const winPng = resvgWin.render().asPng();
  writeFileSync(join(__dirname, "../src-tauri/app-icon-win.png"), winPng);
  console.log("wrote app-icon-win.png (Windows, full canvas, r=100, transparent corners):", winPng.length, "bytes");
}

main();