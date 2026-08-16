import { writeFileSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

function main() {
  // Read the Windows-optimized PNG (white background)
  const winPngPath = join(__dirname, "../src-tauri/app-icon-win.png");
  const png = readFileSync(winPngPath);

  // Wrap PNG in ICO container
  const ico = Buffer.alloc(6 + 16 + png.length);
  let offset = 0;

  // ICO header
  ico.writeUInt16LE(0, offset); offset += 2;  // Reserved
  ico.writeUInt16LE(1, offset); offset += 2;  // Type: 1 = ICO
  ico.writeUInt16LE(1, offset); offset += 2;  // Count: 1 image

  // Directory entry
  ico.writeUInt8(0, offset); offset += 1;     // Width: 0 = 256
  ico.writeUInt8(0, offset); offset += 1;     // Height: 0 = 256
  ico.writeUInt8(0, offset); offset += 1;     // Color count
  ico.writeUInt8(0, offset); offset += 1;     // Reserved
  ico.writeUInt16LE(1, offset); offset += 2;  // Planes
  ico.writeUInt16LE(32, offset); offset += 2; // Bit count
  ico.writeUInt32LE(png.length, offset); offset += 4;  // Size of PNG data
  ico.writeUInt32LE(22, offset); offset += 4; // Offset to PNG data (6 + 16)

  // PNG data
  png.copy(ico, offset);

  const icoOut = join(__dirname, "../src-tauri/icons/icon.ico");
  writeFileSync(icoOut, ico);
  console.log("wrote icon.ico (Windows, from white-bg PNG):", ico.length, "bytes");
}

main();