import { deflateSync } from "node:zlib";
import { writeFileSync } from "node:fs";

const W = 1024;
const H = 1024;
const R = 180;

function makeTable() {
  const t = new Int32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c;
  }
  return t;
}
const TABLE = makeTable();

function crc32(buf) {
  let crc = -1;
  for (let i = 0; i < buf.length; i++) {
    crc = TABLE[(crc ^ buf[i]) & 0xff] ^ (crc >>> 8);
  }
  return (crc ^ -1) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const typeBuf = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])));
  return Buffer.concat([len, typeBuf, data, crc]);
}

const raw = Buffer.alloc(H * (1 + W * 4));
for (let y = 0; y < H; y++) {
  const row = y * (1 + W * 4);
  raw[row] = 0;
  for (let x = 0; x < W; x++) {
    const dx = Math.max(R - x, x - (W - 1 - R), 0);
    const dy = Math.max(R - y, y - (H - 1 - R), 0);
    const d = Math.sqrt(dx * dx + dy * dy);
    let alpha = 1;
    if (d > R) alpha = 0;
    else if (d > R - 2) alpha = (R - d) / 2;

    const t = y / H;
    const rr = Math.round(0x2f + (0x7b - 0x2f) * t);
    const gg = Math.round(0x5e + (0x5c - 0x5e) * t);
    const bb = Math.round(0xc8 + (0xff - 0xc8) * t);

    const o = row + 1 + x * 4;
    raw[o] = rr;
    raw[o + 1] = gg;
    raw[o + 2] = bb;
    raw[o + 3] = Math.round(255 * alpha);
  }
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(W, 0);
ihdr.writeUInt32BE(H, 4);
ihdr[8] = 8;
ihdr[9] = 6;
ihdr[10] = 0;
ihdr[11] = 0;
ihdr[12] = 0;

const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", ihdr),
  chunk("IDAT", deflateSync(raw, { level: 9 })),
  chunk("IEND", Buffer.alloc(0)),
]);

const out = new URL("../src-tauri/app-icon.png", import.meta.url);
writeFileSync(out, png);
console.log("wrote app-icon.png:", png.length, "bytes");
