// Screenshots der gebauten App (pnpm build vorher ausführen).
// Startet einen kleinen statischen Server auf dist/ und fährt die Zustände mit Playwright ab.
// Aufruf: pnpm shots [filter]   – z. B. `pnpm shots 1280` oder `pnpm shots schutz`
// Misst dabei nach: Bühnenhöhe, Höhe der Bedienziele, sichtbare Listenzeilen.
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
// Zustände der Ansicht „Mixen" (MockCore, deterministisch).
const SCENES = [
  ['empty', 'leer'],
  ['loaded', 'geladen'],
  ['synced', 'synchron'],
  ['transition', 'uebergang'],
];
/** Grenzwerte der Nachprüfung. */
const MIN_STAGE = 0.4; // Anteil der Fensterhöhe
const MIN_TARGET = 28; // px, Bedienziele im Live-Betrieb (9.3)
// Was live geklickt wird: Tasten, Lade-Tasten, Listenzeilen, Sortierköpfe, Regler, Fader, Übersicht, Suche, Werte.
const TARGETS = '.dw-key, .lib__load, .dw-row[data-track], .lib__sort, .dw-knob__dial, .dw-fader, .overview, .lib__search, .hdr__tab, .dw-value, .hdr__warn';

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

/** Maße der Ansicht „Mixen": Bühnenanteil, kleinstes Bedienziel, ganz sichtbare Listenzeilen. */
function measure(page) {
  return page.evaluate((sel) => {
    const stage = document.querySelector('.stage')?.getBoundingClientRect();
    const table = document.querySelector('.lib__table')?.getBoundingClientRect();
    const head = document.querySelector('.lib__table .dw-row[data-head]')?.getBoundingClientRect();
    let rows = 0;
    if (table && head) {
      for (const r of document.querySelectorAll('.lib__table .dw-row[data-track]')) {
        const b = r.getBoundingClientRect();
        if (b.top >= head.bottom - 0.5 && b.bottom <= table.bottom + 0.5) rows += 1;
      }
    }
    const small = [];
    let min = Infinity;
    for (const el of document.querySelectorAll(sel)) {
      const b = el.getBoundingClientRect();
      if (b.width === 0 || b.height === 0) continue;
      if (b.height < min) min = b.height;
      if (b.height < 27.5) small.push(`${el.className.toString().split(' ')[0]} ${Math.round(b.width)}×${Math.round(b.height)}`);
    }
    // Abstand der Lade-Tasten einer Zeile
    const loads = [...document.querySelectorAll('.lib__loads')][0]?.querySelectorAll('.lib__load');
    const loadGap = loads && loads.length > 1 ? loads[1].getBoundingClientRect().left - loads[0].getBoundingClientRect().right : null;
    const load = loads && loads[0] ? loads[0].getBoundingClientRect() : null;
    return {
      stage: stage ? stage.height / window.innerHeight : 0,
      stagePx: stage ? Math.round(stage.height) : 0,
      rows,
      minTarget: Math.round(min * 10) / 10,
      small: [...new Set(small)].slice(0, 10),
      load: load ? `${Math.round(load.width)}×${Math.round(load.height)}, Abstand ${Math.round(loadGap)}` : '-',
    };
  }, TARGETS);
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
  const measures = [];

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
    if (opts.measure) {
      const m = await measure(page);
      measures.push({ name, ...m });
      if (opts.measure.stage && m.stage < MIN_STAGE - 0.0005) errors.push(`${name}: Bühne nur ${(m.stage * 100).toFixed(1)} % der Höhe`);
      if (opts.measure.targets && m.small.length) errors.push(`${name}: Bedienziele unter ${MIN_TARGET} px: ${m.small.join(', ')}`);
    }
    made.push(file);
    await context.close();
  };

  for (const [w, h] of SIZES) {
    for (const [scene, label] of SCENES) {
      // Nachmessen im Zustand „Übergang": Bühne ≥ 40 % (720p, 1080p), Bedienziele ≥ 28 px (720p).
      const check = scene === 'transition' ? { stage: h <= 1080, targets: h === 720 } : undefined;
      await shoot(`mixen-${label}-${w}x${h}`, `?scene=${scene}&freeze=1#/mixen`, w, h, { measure: check });
    }
  }
  await shoot('designsystem-1920', '?scene=transition&freeze=1#/designsystem', 1920, 1080, { fullPage: true });

  // --- Zustände der zweiten Runde -------------------------------------------------------------
  // Live-Schutz: Stopp-Versuch am hörbaren Master (Q kurz gedrückt) – Hinweis, Taste scharf.
  await shoot('zustand-schutz-stopp-1280x720', '?scene=transition&freeze=1#/mixen', 1280, 720, {
    before: async (page) => {
      await page.keyboard.press('KeyQ');
      await page.waitForTimeout(150);
    },
  });
  // Leere Bibliothek mit Import-Hinweis.
  await shoot('zustand-bibliothek-leer-1280x720', '?scene=empty&library=empty&freeze=1#/mixen', 1280, 720);
  // Bibliothek mit Tracks in der Analyse und einer fehlenden Datei.
  await shoot('zustand-analyse-1280x720', '?scene=loaded&library=analyzing&freeze=1#/mixen', 1280, 720, {
    before: async (page) => {
      // Nach BPM sortiert stehen die noch nicht analysierten Tracks am Ende: dorthin rollen.
      await page.locator('.lib__sort', { hasText: 'BPM' }).click();
      await page.evaluate(() => {
        const table = document.querySelector('.lib__table');
        if (table) table.scrollTop = table.scrollHeight;
      });
      await page.mouse.move(640, 300);
      await page.waitForTimeout(150);
    },
  });
  // Kein Audiogerät: Warnhinweis in der Kopfzeile, Meldung in der Kurzhilfe.
  await shoot('zustand-kein-audio-1280x720', '?scene=synced&device=none&freeze=1#/mixen', 1280, 720, {
    before: async (page) => {
      await page.locator('.hdr__warn').hover();
      await page.waitForTimeout(700);
    },
  });
  // Laden dauert: Ladezustand an Deck B.
  await shoot('zustand-laden-1280x720', '?scene=loaded&loaddelay=60000&freeze=1#/mixen', 1280, 720, {
    before: async (page) => {
      const btn = page.locator('.lib__table [data-track]').nth(3).locator('.lib__load[data-deck=b]');
      await btn.click();
      await page.mouse.move(640, 300);
      await page.waitForTimeout(300);
    },
  });
  // Fehler beim Laden: Klartext der Hülle als Hinweis.
  await shoot('zustand-ladefehler-1280x720', '?scene=loaded&library=analyzing&freeze=1#/mixen', 1280, 720, {
    before: async (page) => {
      await page.evaluate(() => {
        // Die fehlende Datei per Doppelklick laden wollen.
        const row = [...document.querySelectorAll('.lib__table [data-track]')].find((r) => r.getAttribute('data-dim') === 'true');
        row?.scrollIntoView({ block: 'center' });
        row?.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
      });
      await page.waitForTimeout(200);
    },
  });
  // Bibliothek aufgezogen: Decks als eine Zeile.
  await shoot('zustand-bibliothek-gross-1280x720', '?scene=transition&freeze=1&lib=big#/mixen', 1280, 720, { measure: { stage: true, targets: true } });
  // Sprung in der Übersicht am hörbaren Deck ohne Shift: Hinweis.
  await shoot('zustand-schutz-sprung-1280x720', '?scene=transition&freeze=1#/mixen', 1280, 720, {
    before: async (page) => {
      const box = await page.locator('.deck[data-deck=a] .overview').boundingBox();
      await page.mouse.click(box.x + box.width * 0.3, box.y + box.height / 2);
      await page.waitForTimeout(150);
    },
  });

  // Zusätzliche Prüfbilder: 4K, Canvas-Rückfall, Englisch, Platzhalter, kontrastreiches Design.
  await shoot('extra-mixen-uebergang-3840x2160', '?scene=transition&freeze=1#/mixen', 3840, 2160, { measure: {} });
  await shoot('extra-mixen-canvas2d-1280x720', '?scene=transition&freeze=1&renderer=2d#/mixen', 1280, 720);
  await shoot('extra-mixen-english-1280x720', '?scene=synced&freeze=1&lang=en#/mixen', 1280, 720);
  await shoot('extra-mixen-kontrast-1280x720', '?scene=transition&freeze=1&theme=contrast#/mixen', 1280, 720);
  await shoot('extra-platzhalter-automix-1280x720', '?scene=empty&freeze=1#/automix', 1280, 720);

  await browser.close();
  server.close();
  for (const f of made) console.log(path.relative(ROOT, f));
  if (measures.length) {
    console.log('\nNachmessung:');
    for (const m of measures) {
      console.log(
        `  ${m.name}: Bühne ${m.stagePx} px = ${(m.stage * 100).toFixed(1)} % · kleinstes Bedienziel ${m.minTarget} px · ganze Listenzeilen ${m.rows} · Lade-Taste ${m.load}`,
      );
    }
  }
  if (errors.length) {
    console.error('\nAuffälligkeiten:');
    for (const e of errors) console.error('  ' + e);
    process.exit(1);
  }
  console.log(`\n${made.length} Screenshots in ${path.relative(ROOT, OUT)}/`);
})();
