#!/usr/bin/env node
/**
 * Generate geo_continent_map.bin from a Natural Earth Admin 0 Countries shapefile.
 *
 * Usage:
 *   node tools/generate_geo_data.js <ne_*_admin_0_countries.zip|dir> <output.bin>
 *
 * Default paths (when run from repo root):
 *   input  – ne_110m_admin_0_countries.zip (must be supplied as argv[2])
 *   output – src-tauri/src/scenery/geo_continent_map.bin
 *
 * Output format (64,800 bytes):
 *   flat u8 array, index = (lat + 90) * 360 + (lon + 180)
 *   0 = Ocean/Unknown  1 = Africa  2 = Antarctica  3 = Asia
 *   4 = Europe  5 = North America  6 = Oceania  7 = South America
 *
 * Source data: https://www.naturalearthdata.com/downloads/110m-cultural-vectors/
 *   "Admin 0 – Countries" at 1:110m scale (ne_110m_admin_0_countries)
 */

'use strict';

const fs   = require('fs');
const path = require('path');
const zlib = require('zlib'); // not used directly, but kept for reference

// ── Shapefile zip reader ─────────────────────────────────────────────────────

/** Minimal zip reader (PKZIP central-directory based, no compression). */
function readZipEntries(buf) {
  const entries = {};
  // Scan for local file headers (sig 0x04034b50)
  let i = 0;
  while (i < buf.length - 4) {
    if (buf.readUInt32LE(i) !== 0x04034b50) { i++; continue; }
    const compMethod  = buf.readUInt16LE(i + 8);
    const compSize    = buf.readUInt32LE(i + 18);
    const uncompSize  = buf.readUInt32LE(i + 22);
    const nameLen     = buf.readUInt16LE(i + 26);
    const extraLen    = buf.readUInt16LE(i + 28);
    const name        = buf.slice(i + 30, i + 30 + nameLen).toString('utf8');
    const dataStart   = i + 30 + nameLen + extraLen;

    let data;
    if (compMethod === 0) {
      // Stored (no compression)
      data = buf.slice(dataStart, dataStart + uncompSize);
    } else if (compMethod === 8) {
      // Deflated
      data = zlib.inflateRawSync(buf.slice(dataStart, dataStart + compSize));
    } else {
      data = null; // unsupported
    }
    if (data) entries[name] = data;
    i = dataStart + compSize;
  }
  return entries;
}

// ── DBF parser ───────────────────────────────────────────────────────────────

function readDbf(buf) {
  const numRecords = buf.readUInt32LE(4);
  const headerSize = buf.readUInt16LE(8);
  const recordSize = buf.readUInt16LE(10);

  const fields = [];
  let pos = 32;
  while (pos < headerSize - 1) {
    if (buf[pos] === 0x0d) break;
    const name = buf.slice(pos, pos + 11).toString('ascii').replace(/\0/g, '').trim();
    const flen = buf[pos + 16];
    fields.push({ name, len: flen });
    pos += 32;
  }

  const records = [];
  pos = headerSize;
  for (let r = 0; r < numRecords; r++) {
    if (pos >= buf.length) break;
    const deleted = buf[pos]; pos++;
    const rec = {};
    for (const f of fields) {
      const raw = buf.slice(pos, pos + f.len);
      rec[f.name] = raw.toString('utf8').replace(/\0/g, '').trim();
      pos += f.len;
    }
    if (deleted !== 0x2a) records.push(rec); // skip deleted
  }
  return records;
}

// ── SHP parser (polygon, shape type 5) ───────────────────────────────────────

function readShp(buf) {
  const shapes = [];
  let pos = 100; // skip file header
  while (pos + 8 <= buf.length) {
    const contentLen  = buf.readUInt32BE(pos + 4); // in 16-bit words
    pos += 8;
    const contentStart = pos;
    const shapeType   = buf.readUInt32LE(pos); pos += 4;

    if (shapeType === 0) { pos = contentStart + contentLen * 2; continue; }
    if (shapeType !== 5) { pos = contentStart + contentLen * 2; continue; }

    const bbox = [
      buf.readDoubleBE ? undefined : null,
      // read as LE doubles
      buf.readDoubleLE(pos),     // xmin
      buf.readDoubleLE(pos + 8), // ymin
      buf.readDoubleLE(pos + 16),// xmax
      buf.readDoubleLE(pos + 24),// ymax
    ];
    const xmin = buf.readDoubleLE(pos);
    const ymin = buf.readDoubleLE(pos + 8);
    const xmax = buf.readDoubleLE(pos + 16);
    const ymax = buf.readDoubleLE(pos + 24);
    pos += 32;

    const numParts  = buf.readUInt32LE(pos);
    const numPoints = buf.readUInt32LE(pos + 4);
    pos += 8;

    const parts = [];
    for (let p = 0; p < numParts; p++) { parts.push(buf.readUInt32LE(pos)); pos += 4; }

    const pts = [];
    for (let p = 0; p < numPoints; p++) {
      pts.push([buf.readDoubleLE(pos), buf.readDoubleLE(pos + 8)]);
      pos += 16;
    }

    shapes.push({ xmin, ymin, xmax, ymax, parts, pts });
    pos = contentStart + contentLen * 2;
  }
  return shapes;
}

// ── Point-in-polygon (ray casting) ───────────────────────────────────────────

function pip(px, py, poly) {
  const n = poly.length;
  if (n < 3) return false;
  let inside = false;
  let [x1, y1] = poly[0];
  for (let i = 1; i <= n; i++) {
    const [x2, y2] = poly[i % n];
    if (Math.min(y1, y2) < py && py <= Math.max(y1, y2)) {
      if (px <= Math.max(x1, x2)) {
        const xi = y1 !== y2 ? (py - y1) * (x2 - x1) / (y2 - y1) + x1 : x1;
        if (x1 === x2 || px <= xi) inside = !inside;
      }
    }
    x1 = x2; y1 = y2;
  }
  return inside;
}

// ── Main ─────────────────────────────────────────────────────────────────────

const CONTINENTS = {
  Africa: 1, Antarctica: 2, Asia: 3, Europe: 4,
  'North America': 5, Oceania: 6, 'South America': 7,
  // Natural Earth marks these island territories as "Seven seas (open ocean)".
  // Override them to their geographically appropriate continent.
  'Seven seas (open ocean)': 0, // default — overridden per-country below
};

// Per-country continent overrides (keyed by ADMIN field).
// Used when CONTINENT is "Seven seas (open ocean)" or otherwise unmapped.
const COUNTRY_CONTINENT_OVERRIDES = {
  'Seychelles':                           1, // Africa
  'Mauritius':                            1, // Africa
  'Saint Helena':                         1, // Africa
  'British Indian Ocean Territory':       3, // Asia
  'Maldives':                             3, // Asia
  'South Georgia and the Islands':        7, // South America
  'French Southern and Antarctic Lands':  2, // Antarctica
  'Heard Island and McDonald Islands':    2, // Antarctica
};

const CONTINENT_NAMES = ['Ocean','Africa','Antarctica','Asia','Europe','North America','Oceania','South America'];

const inputPath = process.argv[2];
const outPath = process.argv[3] || path.join(__dirname, '..', 'src-tauri', 'src', 'scenery', 'geo_continent_map.bin');

if (!inputPath) {
  console.error('Usage: node generate_geo_data.js <ne_*_admin_0_countries.zip|dir> [output.bin]');
  process.exit(1);
}

// Support both a .zip file and an already-extracted directory
let shpBuf, dbfBuf;
const stat = fs.statSync(inputPath);
if (stat.isDirectory()) {
  const files = fs.readdirSync(inputPath);
  const shpFile = files.find(f => f.toLowerCase().endsWith('.shp'));
  const dbfFile = files.find(f => f.toLowerCase().endsWith('.dbf'));
  if (!shpFile || !dbfFile) { console.error('No .shp/.dbf found in directory'); process.exit(1); }
  console.log(`Reading directory ${inputPath} ...`);
  console.log(`  ${shpFile}, ${dbfFile}`);
  shpBuf = fs.readFileSync(path.join(inputPath, shpFile));
  dbfBuf = fs.readFileSync(path.join(inputPath, dbfFile));
} else {
  console.log(`Reading ${inputPath} ...`);
  const zipBuf  = fs.readFileSync(inputPath);
  const entries = readZipEntries(zipBuf);
  const shpKey = Object.keys(entries).find(k => k.toLowerCase().endsWith('.shp'));
  const dbfKey = Object.keys(entries).find(k => k.toLowerCase().endsWith('.dbf'));
  if (!shpKey || !dbfKey) { console.error('Could not find .shp/.dbf inside zip'); process.exit(1); }
  console.log(`  ${shpKey}, ${dbfKey}`);
  shpBuf = entries[shpKey];
  dbfBuf = entries[dbfKey];
}

console.log(`  Parsing shapes + records ...`);
const shapes  = readShp(shpBuf);
const records = readDbf(dbfBuf);
console.log(`  ${shapes.length} shapes, ${records.length} records`);

// Pair each shape with its continent ID
const sc = [];
for (let i = 0; i < Math.min(shapes.length, records.length); i++) {
  const rec = records[i];
  const cid = COUNTRY_CONTINENT_OVERRIDES[rec.ADMIN] ?? CONTINENTS[rec.CONTINENT] ?? 0;
  if (cid) sc.push({ sh: shapes[i], cid });
}
console.log(`  ${sc.length} land shapes with known continent`);

// Generate lookup
const lookup = Buffer.alloc(180 * 360, 0);
let land = 0;
for (let li = 0; li < 180; li++) {
  const lat  = li - 90;
  const tlat = lat + 0.5;
  if (li % 30 === 0) process.stdout.write(`  lat ${lat >= 0 ? '+' : ''}${lat} ...\n`);

  for (let lo = 0; lo < 360; lo++) {
    const lon  = lo - 180;
    const tlon = lon + 0.5;
    const idx  = li * 360 + lo;

    for (const { sh, cid } of sc) {
      if (tlon < sh.xmin || tlon > sh.xmax || tlat < sh.ymin || tlat > sh.ymax) continue;

      const ends = [...sh.parts.slice(1), sh.pts.length];
      let hit = false;
      for (let p = 0; p < sh.parts.length; p++) {
        const part = sh.pts.slice(sh.parts[p], ends[p]);
        if (pip(tlon, tlat, part)) { hit = true; break; }
      }
      if (hit) { lookup[idx] = cid; break; }
    }
    if (lookup[idx]) land++;
  }
}

console.log(`Land cells: ${land} / 64800 (${(land / 648).toFixed(1)}%)`);
fs.writeFileSync(outPath, lookup);
console.log(`Written ${lookup.length} bytes → ${outPath}`);

// Stats
const cnt = {};
for (const b of lookup) cnt[b] = (cnt[b] || 0) + 1;
for (const k of Object.keys(cnt).sort((a,b) => +a - +b)) {
  console.log(`  ${CONTINENT_NAMES[+k]}: ${cnt[k]} cells`);
}
