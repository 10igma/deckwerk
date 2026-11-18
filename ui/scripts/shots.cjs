// Screenshots der gebauten App (pnpm build vorher ausführen).
// Startet einen kleinen statischen Server auf dist/ und fährt die Zustände mit Playwright ab.
// Aufruf: pnpm shots [filter]   – z. B. `pnpm shots 1280` oder `pnpm shots transition`
const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');
const { chromium } = require('playwright');

const ROOT = path.resolve(__dirname, '..');
const DIST = path.join(ROOT, 'dist');
const OUT = path.join(ROOT, '.shots');
const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json',
  '.woff2': 'font/woff2',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
};

const SIZES = [
  [1280, 720],
  [1920, 1080],
  [2560, 1440],
];
// Zustände der Ansicht „Mixen“ (MockCore, deterministisch).
const SCENES = [
  ['empty', 'leer'],
  ['loaded', 'geladen'],
  ['synced', 'synchron'],
  ['transition', 'uebergang'],
];

function serve() {
  const server = http.createServer((req, res) => {
    const url = new URL(req.url, 'http://localhost');
    let file = path.join(DIST, decodeURIComponent(url.pathname));
    if (!file.startsWith(DIST)) {
      res.writeHead(403).end();
      return;
    }
    if (fs.existsSync(file) && fs.statSync(file).isDirectory()) file = path.join(file, 'index.html');
    if (!fs.existsSync(file)) {
      res.writeHead(404).end('not found');
      return;
    }
    res.writeHead(200, { 'Content-Type': MIME[path.extname(file)] || 'application/octet-stream' });
    fs.createReadStream(file).pipe(res);
  });
  return new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve(server)));
}

async function settle(page) {
  // Schriften geladen, Wellenform gezeichnet, Pegel eingeschwungen.
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(700);
}

(async () => {
  if (!fs.existsSync(path.join(DIST, 'index.html'))) {
    console.error('dist/ fehlt – zuerst `pnpm build` ausführen.');
    process.exit(1);
  }
  const filter = process.argv[2] || '';
  fs.mkdirSync(OUT, { recursive: true });
  const server = await serve();
  const base = `http://127.0.0.1:${server.address().port}`;
  const browser = await chromium.launch();
  const errors = [];
  const made = [];

  const shoot = async (name, url, width, height, opts = {}) => {
    if (filter && !name.includes(filter)) return;
    const context = await browser.newContext({ viewport: { width, height }, deviceScaleFactor: 1 });
    const page = await context.newPage();
    page.on('console', (m) => {
      // Treiber-Hinweis von Headless-Chromium beim Auslesen des Bildes ist kein Fehler der App.
      if (m.text().includes('GL Driver Message')) return;
      if (m.type() === 'error' || m.type() === 'warning') errors.push(`${name}: [${m.type()}] ${m.text()}`);
    });
    page.on('pageerror', (e) => errors.push(`${name}: ${e.message}`));
    await page.goto(`${base}/${url}`);
    await settle(page);
    if (opts.before) await opts.before(page);
    const file = path.join(OUT, `${name}.png`);
    await page.screenshot({ path: file, fullPage: !!opts.fullPage });
    // Prüfung: nichts darf über das Fenster hinausragen (VIS-10).
    if (!opts.fullPage) {
      const overflow = await page.evaluate(() => {
        const out = [];
        const vw = window.innerWidth;
        const vh = window.innerHeight;
        for (const el of document.querySelectorAll('.app *')) {
          const r = el.getBoundingClientRect();
          if (r.width === 0 || r.height === 0) continue;
          if (el.closest('.lib__table') && !el.matches('.lib__table')) continue;
          if (r.right > vw + 0.5 || r.bottom > vh + 0.5 || r.left < -0.5 || r.top < -0.5) {
            out.push(`${el.tagName.toLowerCase()}.${String(el.className.baseVal ?? el.className).split(' ')[0]}`);
          }
        }
        return [...new Set(out)].slice(0, 8);
      });
      if (overflow.length) errors.push(`${name}: ragt über das Fenster hinaus: ${overflow.join(', ')}`);
    }
    made.push(file);
    await context.close();
  };

  for (const [w, h] of SIZES) {
    for (const [scene, label] of SCENES) {
      await shoot(`mixen-${label}-${w}x${h}`, `?scene=${scene}&freeze=1#/mixen`, w, h);
    }
  }
  await shoot('designsystem-1920', '?scene=transition&freeze=1#/designsystem', 1920, 1080, { fullPage: true });

  // Zusätzliche Prüfbilder: 4K, Canvas-Rückfall, Englisch, Platzhalter.
  await shoot('extra-mixen-uebergang-3840x2160', '?scene=transition&freeze=1#/mixen', 3840, 2160);
  await shoot('extra-mixen-canvas2d-1280x720', '?scene=transition&freeze=1&renderer=2d#/mixen', 1280, 720);
  await shoot('extra-mixen-english-1280x720', '?scene=synced&freeze=1&lang=en#/mixen', 1280, 720);
  await shoot('extra-platzhalter-automix-1280x720', '?scene=empty&freeze=1#/automix', 1280, 720);

  await browser.close();
  server.close();
  for (const f of made) console.log(path.relative(ROOT, f));
  if (errors.length) {
    console.error('\nAuffälligkeiten:');
    for (const e of errors) console.error('  ' + e);
    process.exit(1);
  }
  console.log(`\n${made.length} Screenshots in ${path.relative(ROOT, OUT)}/`);
})();
