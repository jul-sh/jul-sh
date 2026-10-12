// Technical packaging of the generated 5×3 sheets; no poses are synthesized here.
// Requires Sharp. Run with: node scripts/pack-illustration-frames.cjs
const sharp = require('sharp');
const fs = require('node:fs/promises');
const path = require('node:path');
const crypto = require('node:crypto');

const root = path.join(__dirname, '..');
const sourceDir = path.join(root, 'artwork/animations');
const outputDir = path.join(root, 'static/animations');

function separators(projection, count) {
  const seams = [0];
  for (let i = 1; i < count; i++) {
    const expected = projection.length * i / count;
    const radius = projection.length / count * .2;
    let best = Math.round(expected);
    for (let x = Math.ceil(expected - radius); x < expected + radius; x++) {
      if (projection[x] < projection[best] ||
          (projection[x] === projection[best] && Math.abs(x - expected) < Math.abs(best - expected))) best = x;
    }
    // Center a clear gutter so neither neighboring pose is cropped.
    let left = best, right = best;
    while (left > expected - radius && projection[left - 1] === projection[best]) left--;
    while (right < expected + radius && projection[right + 1] === projection[best]) right++;
    seams.push(Math.round((left + right) / 2));
  }
  return [...seams, projection.length];
}

async function pack(city, frameWidth, frameHeight, sourceName = `${city}-15-source.png`) {
  const source = path.join(sourceDir, sourceName);
  const {data, info} = await sharp(source).ensureAlpha().raw().toBuffer({resolveWithObject:true});
  const alpha = (x, y) => data[(y * info.width + x) * 4 + 3];
  const xs = Array(info.width).fill(0), ys = Array(info.height).fill(0);
  for (let y = 0; y < info.height; y++) for (let x = 0; x < info.width; x++) {
    if (alpha(x, y) >= 128) { xs[x]++; ys[y]++; }
  }
  const columns = separators(xs, 5);
  // The generated roar atlas has extra space above its first row.
  const rows = city === 'bear-roar' ? [0, 533, 840, info.height] : separators(ys, 3);
  const frames = [];
  for (let i = 0; i < 15; i++) {
    const col = i % 5, row = Math.floor(i / 5);
    let left = columns[col + 1], right = columns[col], top = rows[row + 1], bottom = rows[row];
    for (let y = rows[row]; y < rows[row + 1]; y++) for (let x = columns[col]; x < columns[col + 1]; x++) {
      if (alpha(x, y) >= 64) {
        left = Math.min(left, x); right = Math.max(right, x);
        top = Math.min(top, y); bottom = Math.max(bottom, y);
      }
    }
    if (right <= left || bottom <= top) throw new Error(`${city}: missing frame ${i + 1}`);
    let baseLeft = right, baseRight = left;
    for (let y = Math.round(bottom - (bottom - top) * .28); y <= bottom; y++) for (let x = left; x <= right; x++) {
      if (alpha(x, y) >= 128) { baseLeft = Math.min(baseLeft, x); baseRight = Math.max(baseRight, x); }
    }
    let anchorX = (baseLeft + baseRight) / 2;
    // Keep the back of the neck fixed while the muzzle tilts and the jaw opens.
    if (city === 'bear-roar') anchorX = baseRight;
    frames.push({left, top, width:right - left + 1, height:bottom - top + 1, anchorX});
  }
  const leftExtent = Math.max(...frames.map(f => f.anchorX - f.left));
  const rightExtent = Math.max(...frames.map(f => f.left + f.width - f.anchorX));
  const padding = 3;
  const scale = Math.min((frameWidth - 2 * padding) / (leftExtent + rightExtent),
    (frameHeight - 2 * padding) / Math.max(...frames.map(f => f.height)));
  const center = (frameWidth - (leftExtent + rightExtent) * scale) / 2 + leftExtent * scale;
  const frameDir = path.join(sourceDir, city);
  await fs.mkdir(frameDir, {recursive:true});
  const composites = [], hashes = [];
  for (const [i, f] of frames.entries()) {
    const width = Math.round(f.width * scale), height = Math.round(f.height * scale);
    const pose = await sharp(source).extract({left:f.left, top:f.top, width:f.width, height:f.height})
      .resize(width, height).png().toBuffer();
    const frame = await sharp({create:{width:frameWidth, height:frameHeight, channels:4, background:'#0000'}})
      .composite([{input:pose, left:Math.round(center - (f.anchorX - f.left) * scale), top:frameHeight - padding - height}])
      .png().toBuffer();
    hashes.push(crypto.createHash('sha256').update(frame).digest('hex'));
    await fs.writeFile(path.join(frameDir, `${String(i + 1).padStart(2, '0')}.png`), frame);
    composites.push({input:frame, left:i * frameWidth, top:0});
  }
  if (new Set(hashes).size !== 15) throw new Error(`${city}: frames must be distinct`);
  const destination = path.join(outputDir, `${city}-15.webp`);
  await sharp({create:{width:frameWidth * 15, height:frameHeight, channels:4, background:'#0000'}})
    .composite(composites).webp({quality:92, alphaQuality:100, effort:6}).toFile(destination);
  const manifest = {frameCount:15, frameWidth, frameHeight, durationMs:1500,
    layout:'horizontal strip', source:path.basename(source), columns, rows, scale, frames, hashes};
  await fs.writeFile(path.join(sourceDir, `${city}-15-packed.json`), JSON.stringify(manifest, null, 2) + '\n');
  console.log(`${city}: 15 distinct ${frameWidth}×${frameHeight} frames, ${(await fs.stat(destination)).size} bytes`);
}

(async () => {
  await fs.mkdir(outputDir, {recursive:true});
  for (const args of [['liberty-flame',128,96], ['bus',192,128], ['bear-roar',192,192]]) await pack(...args);
})().catch(error => { console.error(error); process.exitCode = 1; });
