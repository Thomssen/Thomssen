// Generates installer/psychosec.ico (PNG-compressed multi-size ICO) and apps/website/public/icon.png.
// Pure Node, no dependencies:  node scripts/make-icon.mjs
import { deflateSync } from "node:zlib";
import { writeFileSync, mkdirSync } from "node:fs";

const BG = [0x08, 0x09, 0x0b], FG = [0x87, 0xbd, 0xbb], EDGE = [0x29, 0x2b, 0x2f];

function inside(x, y) {
  // original "eye" mark: lens outline with a pupil
  const dx = Math.abs(x - 0.5) * 2, dy = Math.abs(y - 0.5);
  const lens = dx <= 1 && dy <= 0.27 * (1 - dx ** 1.7) + 0.0;
  return lens && dx > -1;
}
function eye(x, y) {
  const d = Math.hypot(x - 0.5, y - 0.5);
  return d > 0.085 && d < 0.14;
}
function roundedBg(x, y) {
  const rad = 0.18, dx = Math.max(Math.abs(x - 0.5) - (0.5 - rad), 0), dy = Math.max(Math.abs(y - 0.5) - (0.5 - rad), 0);
  return dx * dx + dy * dy <= rad * rad;
}
function render(size) {
  const px = Buffer.alloc(size * size * 4), S = 4;
  for (let j = 0; j < size; j++) for (let i = 0; i < size; i++) {
    let rr = 0, gg = 0, bb = 0, aa = 0;
    for (let sj = 0; sj < S; sj++) for (let si = 0; si < S; si++) {
      const x = (i + (si + 0.5) / S) / size, y = (j + (sj + 0.5) / S) / size;
      let c = null;
      if (roundedBg(x, y)) c = BG;
      if (c && inside(x, y)) c = eye(x, y) ? BG : FG;
      if (c) { rr += c[0]; gg += c[1]; bb += c[2]; aa += 1; }
    }
    const o = (j * size + i) * 4, n = S * S;
    if (aa) { px[o] = rr / aa; px[o + 1] = gg / aa; px[o + 2] = bb / aa; }
    px[o + 3] = Math.round((aa / n) * 255);
  }
  return px;
}
const crcTable = Array.from({ length: 256 }, (_, n) => { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; return c >>> 0; });
const crc = (b) => { let c = 0xffffffff; for (const x of b) c = crcTable[(c ^ x) & 255] ^ (c >>> 8); return (c ^ 0xffffffff) >>> 0; };
function chunk(type, data) {
  const t = Buffer.from(type), len = Buffer.alloc(4), c = Buffer.alloc(4);
  len.writeUInt32BE(data.length); c.writeUInt32BE(crc(Buffer.concat([t, data])));
  return Buffer.concat([len, t, data, c]);
}
function png(size) {
  const px = render(size), raw = Buffer.alloc((size * 4 + 1) * size);
  for (let y = 0; y < size; y++) { raw[y * (size * 4 + 1)] = 0; px.copy(raw, y * (size * 4 + 1) + 1, y * size * 4, (y + 1) * size * 4); }
  const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(size, 0); ihdr.writeUInt32BE(size, 4); ihdr[8] = 8; ihdr[9] = 6;
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk("IHDR", ihdr), chunk("IDAT", deflateSync(raw, { level: 9 })), chunk("IEND", Buffer.alloc(0))]);
}
const sizes = [16, 32, 48, 256], imgs = sizes.map(png);
const head = Buffer.alloc(6); head.writeUInt16LE(1, 2); head.writeUInt16LE(sizes.length, 4);
let off = 6 + 16 * sizes.length;
const dir = Buffer.concat(sizes.map((s, i) => {
  const e = Buffer.alloc(16); e[0] = s === 256 ? 0 : s; e[1] = s === 256 ? 0 : s; e.writeUInt16LE(1, 4); e.writeUInt16LE(32, 6);
  e.writeUInt32LE(imgs[i].length, 8); e.writeUInt32LE(off, 12); off += imgs[i].length; return e;
}));
mkdirSync("installer", { recursive: true });
writeFileSync("installer/psychosec.ico", Buffer.concat([head, dir, ...imgs]));
mkdirSync("apps/website/public", { recursive: true });
writeFileSync("apps/website/public/icon.png", imgs[3]);
mkdirSync("apps/admin/public", { recursive: true });
writeFileSync("apps/admin/public/icon.png", imgs[3]);
console.log("wrote installer/psychosec.ico and website/admin icons");
