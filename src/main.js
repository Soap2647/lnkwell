import '@fontsource-variable/inter';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import * as pdfjs from 'pdfjs-dist';
import workerUrl from 'pdfjs-dist/build/pdf.worker.min.mjs?url';

pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

const $ = (id) => document.getElementById(id);

/** Tiny DOM builder. Text always goes in as text nodes, never as HTML. */
function h(tag, props = {}, ...children) {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (k === 'style') for (const [p, val] of Object.entries(v)) el.style.setProperty(p, val);
    else if (k.startsWith('on')) el.addEventListener(k.slice(2), v);
    else el[k] = v;
  }
  el.append(...children.flat().filter((c) => c != null && c !== false));
  return el;
}

// Static, hand-written icon paths (never user or printer data).
const ICONS = {
  drop: '<path d="M12 3s6 6.200 6 10.500a6 6 0 0 1-12 0C6 9.200 12 3 12 3z"/>',
  refresh: '<path d="M20 11a8 8 0 1 0-2.300 5.700"/><path d="M20 4v7h-7"/>',
  info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v5M12 7.500v.500"/>',
  warn: '<path d="M12 4 2.500 20h19z"/><path d="M12 10v4.500M12 17v.500"/>',
  report: '<rect x="5" y="3" width="14" height="18" rx="2"/><path d="M9 8h6M9 12h6M9 16h3"/>',
  printer: '<path d="M7 8V4h10v4"/><rect x="3" y="8" width="18" height="9" rx="2"/><path d="M7 14h10v6H7z"/>',
};
function icon(name, size = 16, className = '') {
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.setAttribute('viewBox', '0 0 24 24');
  svg.setAttribute('width', size);
  svg.setAttribute('height', size);
  svg.setAttribute('class', `icon ${className}`);
  svg.innerHTML = ICONS[name];
  return svg;
}

let toastTimer;
function toast(text, isError = false) {
  const el = $('toast');
  el.textContent = text;
  el.className = `toast${isError ? ' err' : ''}`;
  el.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (el.hidden = true), 4500);
}

// ---------- Window chrome ----------

const win = getCurrentWindow();
$('winMin').addEventListener('click', () => win.minimize());
$('winMax').addEventListener('click', () => win.toggleMaximize());
$('winClose').addEventListener('click', () => win.close());
$('themeBtn').addEventListener('click', () => {
  const root = document.documentElement;
  root.dataset.theme = root.dataset.theme === 'dark' ? 'light' : 'dark';
  localStorage.setItem('theme', root.dataset.theme);
});

// ---------- PDF preview ----------

const MAX_PREVIEW_PAGES = 20;
let loadToken = 0;

async function openPdf(file) {
  if (!file) return;
  if (file.type !== 'application/pdf' && !file.name.toLowerCase().endsWith('.pdf')) {
    toast('Şimdilik yalnızca PDF açılabiliyor.', true);
    return;
  }
  const token = ++loadToken;
  const pages = $('pages');
  try {
    const pdf = await pdfjs.getDocument({ data: await file.arrayBuffer() }).promise;
    if (token !== loadToken) return;
    pages.replaceChildren();
    $('docName').textContent = file.name;
    $('docPages').textContent =
      pdf.numPages > MAX_PREVIEW_PAGES ? `${pdf.numPages} sayfa · ilk ${MAX_PREVIEW_PAGES}` : `${pdf.numPages} sayfa`;
    $('empty').hidden = true;
    $('doc').hidden = false;
    $('printBtn').disabled = false;
    for (let n = 1; n <= Math.min(pdf.numPages, MAX_PREVIEW_PAGES); n++) {
      const page = await pdf.getPage(n);
      if (token !== loadToken) return;
      const viewport = page.getViewport({ scale: 2 });
      const canvas = h('canvas', { width: viewport.width, height: viewport.height });
      pages.append(canvas);
      await page.render({ canvas, viewport }).promise;
    }
  } catch {
    if (token === loadToken) toast('Bu PDF açılamadı.', true);
  }
}

function closePdf() {
  loadToken++;
  $('pages').replaceChildren();
  $('doc').hidden = true;
  $('empty').hidden = false;
  $('printBtn').disabled = true;
}

const stage = $('stage');
stage.addEventListener('dragover', (e) => {
  e.preventDefault();
  stage.classList.add('dragging');
});
stage.addEventListener('dragleave', (e) => {
  if (!stage.contains(e.relatedTarget)) stage.classList.remove('dragging');
});
stage.addEventListener('drop', (e) => {
  e.preventDefault();
  stage.classList.remove('dragging');
  openPdf(e.dataTransfer.files[0]);
});
// Dropping a file outside the stage must not navigate the window away.
window.addEventListener('dragover', (e) => e.preventDefault());
window.addEventListener('drop', (e) => e.preventDefault());

$('pickBtn').addEventListener('click', () => $('fileInput').click());
$('fileInput').addEventListener('change', (e) => {
  openPdf(e.target.files[0]);
  e.target.value = '';
});
$('closeDoc').addEventListener('click', closePdf);

// ---------- Print settings ----------

const checked = (name) => document.querySelector(`input[name="${name}"]:checked`).value;
const copies = () => Math.min(99, Math.max(1, parseInt($('copies').value, 10) || 1));
const DUPLEX_TEXT = { oneSided: 'tek taraf', flipLikeBook: 'kitap gibi çift taraf', flipLikeCalendar: 'takvim gibi çift taraf' };

function syncSettings() {
  const color = checked('color') === 'true';
  $('copies').value = copies();
  $('pages').classList.toggle('mono', !color);
  $('summary').textContent = [
    `${copies()} kopya`,
    $('media').selectedOptions[0].textContent,
    color ? 'renkli' : 'siyah-beyaz',
    DUPLEX_TEXT[checked('duplex')],
  ].join(' · ');
}
document.querySelector('.inspector').addEventListener('change', syncSettings);
$('copiesDown').addEventListener('click', () => (($('copies').value = copies() - 1), syncSettings()));
$('copiesUp').addEventListener('click', () => (($('copies').value = copies() + 1), syncSettings()));
syncSettings();

$('printBtn').addEventListener('click', async () => {
  const settings = {
    copies: copies(),
    color: checked('color') === 'true',
    quality: checked('quality'),
    duplex: checked('duplex'),
    media: $('media').value,
  };
  try {
    await invoke('print_document', { host: currentHost ?? '', settings });
  } catch {
    // The core's print module is a stub in this draft.
    toast('Baskı gönderimi bu taslakta henüz yok. Ayarların hazır, gönderim sonraki sürümde.');
  }
});

window.addEventListener('keydown', (e) => {
  if (!e.ctrlKey && !e.metaKey) return;
  const key = e.key.toLowerCase();
  if (key === 'o') {
    e.preventDefault();
    $('fileInput').click();
  } else if (key === 'p') {
    e.preventDefault();
    if (!$('printBtn').disabled) $('printBtn').click();
  }
});

// ---------- Printer rail ----------

const STATE_TEXT = {
  ready: 'Hazır',
  processing: 'Yazdırıyor',
  inPowerSave: 'Uyku modunda',
  cancelJob: 'İş iptal ediliyor',
  initializing: 'Başlatılıyor',
  shuttingDown: 'Kapanıyor',
};
const INK_COLOR = {
  C: '#00b4e6', Cyan: '#00b4e6',
  M: '#e5007d', Magenta: '#e5007d',
  Y: '#ffd500', Yellow: '#ffd500',
  K: 'var(--ink-k)', Black: 'var(--ink-k)',
};
const HEAD_NAME = { CMY: 'Renkli kafa', K: 'Siyah kafa' };
const TANK_NAME = { C: 'Camgöbeği', M: 'Macenta', Y: 'Sarı', K: 'Siyah' };
const LEVEL_NAME = { 1: 'Hafif', 2: 'Orta', 3: 'Derin' };
const LEVEL_COST = { 1: 'az mürekkep', 2: 'orta mürekkep', 3: 'çok mürekkep' };

const rail = $('rail');
let currentHost = null;
let lastSnapshot = '';

const inkDot = (key) => h('span', { className: 'ink', style: { background: INK_COLOR[key] ?? 'transparent' } });
const inkDots = (label) => (label === 'CMY' ? ['C', 'M', 'Y'] : [label]).map(inkDot);

function jobLabel(job) {
  if (job.kind.type === 'cleaning') return `${LEVEL_NAME[job.kind.level] ?? `Seviye ${job.kind.level}`} kafa temizliği`;
  if (job.kind.type === 'qualityReport') return 'Baskı kalitesi raporu';
  return job.id;
}

function stateOf(status) {
  const known = status.states.map((s) => STATE_TEXT[s]).filter(Boolean);
  return known.join(' · ') || status.states.join(' · ') || 'Durum bilinmiyor';
}

function worstSeverity(alerts) {
  if (alerts.some((a) => a.severity === 'error')) return 'err';
  if (alerts.some((a) => a.severity === 'warning')) return 'warn';
  return 'ok';
}

/** Same alert repeated per colour becomes one line with several colour dots. */
function groupAlerts(alerts) {
  const groups = new Map();
  for (const a of alerts) {
    const key = `${a.severity}|${a.id}|${a.message ?? ''}`;
    if (!groups.has(key)) groups.set(key, { ...a, colors: [] });
    if (a.color) groups.get(key).colors.push(a.color);
  }
  return [...groups.values()];
}

const section = (title, ...children) => h('section', {}, h('h2', { textContent: title }), ...children);

function tank(c) {
  const known = c.levelMeasured && c.levelPercent != null;
  return h(
    'div',
    { className: 'tank' },
    h(
      'div',
      { className: `tube${known ? '' : ' unknown'}`, style: { '--c': INK_COLOR[c.label] ?? 'var(--text-3)' } },
      h('i', { style: known ? { height: `${c.levelPercent}%` } : {} }),
    ),
    known ? `%${c.levelPercent}` : (TANK_NAME[c.label] ?? c.label),
  );
}

function maintenance(host, jobs) {
  const cleaning = jobs.filter((j) => j.kind.type === 'cleaning').sort((a, b) => a.kind.level - b.kind.level);
  const report = jobs.find((j) => j.kind.type === 'qualityReport');
  const cards = [];

  if (cleaning.length) {
    let selected = cleaning[0];
    const drops = h('span', { className: 'drops' });
    const costText = h('span');
    const showCost = () => {
      drops.replaceChildren(...[1, 2, 3].map((n) => icon('drop', 12, n <= selected.kind.level ? '' : 'off')));
      costText.textContent = LEVEL_COST[selected.kind.level] ?? '';
    };
    showCost();
    cards.push(
      h(
        'div',
        { className: 'care' },
        h(
          'div',
          { className: 'care-head' },
          h('span', { className: 'care-icon' }, icon('drop')),
          h('div', {}, h('div', { className: 'care-title', textContent: 'Kafa temizliği' }), h('div', { className: 'care-sub', textContent: 'Tıkalı püskürtme uçlarını açar' })),
        ),
        h(
          'div',
          { className: 'seg', style: { '--n': cleaning.length }, role: 'radiogroup' },
          cleaning.map((job, i) =>
            h(
              'label',
              {},
              h('input', { type: 'radio', name: 'cleanLevel', checked: i === 0, onchange: () => ((selected = job), showCost()) }),
              h('span', { textContent: LEVEL_NAME[job.kind.level] ?? job.kind.level }),
            ),
          ),
        ),
        h('div', { className: 'cost' }, drops, costText),
        h('button', { className: 'btn wide', textContent: 'Temizliği başlat', onclick: () => confirmJob(host, selected) }),
      ),
    );
  }
  if (report) {
    cards.push(
      h(
        'div',
        { className: 'care' },
        h(
          'div',
          { className: 'care-head' },
          h('span', { className: 'care-icon' }, icon('report')),
          h('div', {}, h('div', { className: 'care-title', textContent: 'Kalite raporu' }), h('div', { className: 'care-sub', textContent: 'Eksik basan rengi gösterir' })),
          h('button', { className: 'btn', textContent: 'Yazdır', onclick: () => confirmJob(host, report) }),
        ),
      ),
    );
  }
  return cards.length ? section('Bakım', ...cards) : null;
}

function renderSnapshot(host, snap) {
  const { device, status, consumables, usage, maintenanceJobs } = snap;
  const severity = worstSeverity(status.alerts);
  const heads = consumables.filter((c) => c.kind === 'printhead');
  const supplies = consumables.filter((c) => c.kind !== 'printhead');
  const alerts = groupAlerts(status.alerts);
  const stats = [
    ['Toplam baskı', usage.totalImpressions],
    ['Renkli', usage.colorImpressions],
    ['Siyah-beyaz', usage.monoImpressions],
    ['Kâğıt sıkışması', usage.jamEvents],
  ].filter(([, v]) => v != null);

  const refreshBtn = h('button', { className: 'mini-btn', title: 'Yenile', ariaLabel: 'Yenile' }, icon('refresh', 14));
  refreshBtn.addEventListener('click', async () => {
    refreshBtn.classList.add('spin');
    await refresh(host, true);
    refreshBtn.classList.remove('spin');
  });

  rail.replaceChildren(
    h(
      'div',
      { className: 'rail-scroll' },
      h(
        'div',
        { className: 'printer-head' },
        h('span', { className: 'status-pill' }, h('span', { className: `dot ${severity}` }), stateOf(status)),
        h('div', { className: 'printer-name', textContent: device.model }),
        h(
          'div',
          { className: 'printer-meta' },
          h('span', { textContent: host }),
          refreshBtn,
          h('button', { className: 'link', textContent: 'Değiştir', onclick: () => renderConnect(host) }),
        ),
      ),
      alerts.length
        ? section(
            'Uyarılar',
            alerts.map((a) =>
              h(
                'div',
                { className: `alert ${a.severity}` },
                icon(a.severity === 'info' ? 'info' : 'warn'),
                a.message ?? a.id,
                h('span', { className: 'inks' }, a.colors.map(inkDot)),
              ),
            ),
          )
        : null,
      supplies.length
        ? section(
            'Mürekkep',
            h('div', { className: 'tanks', style: { 'grid-template-columns': `repeat(${supplies.length}, 1fr)` } }, supplies.map(tank)),
            supplies.some((c) => !c.levelMeasured)
              ? h('p', { className: 'hint', textContent: 'Bu yazıcı tank seviyesini ölçmüyor. Seviyeye tankın üzerindeki çizgilerden bak.' })
              : null,
          )
        : null,
      heads.length
        ? section(
            'Baskı kafaları',
            h(
              'div',
              { className: 'rowlist' },
              heads.map((c) =>
                h(
                  'div',
                  {},
                  h('span', { className: 'inks' }, inkDots(c.label)),
                  HEAD_NAME[c.label] ?? `Kafa (${c.label})`,
                  c.partNumber ? h('span', { className: 'code', textContent: c.partNumber }) : null,
                ),
              ),
            ),
          )
        : null,
      maintenance(host, maintenanceJobs),
      stats.length
        ? section(
            'Sayaçlar',
            h(
              'div',
              { className: 'stats' },
              stats.map(([name, value]) =>
                h('div', { className: 'stat' }, h('b', { textContent: value.toLocaleString('tr-TR') }), h('span', { textContent: name })),
              ),
            ),
          )
        : null,
      h('p', { className: 'hint', textContent: [snap.driver, device.firmware].filter(Boolean).join(' · ') }),
    ),
  );
}

function renderConnect(host = '', error = '') {
  currentHost = null;
  lastSnapshot = '';
  const input = h('input', { value: host, placeholder: '192.0.2.10', autocomplete: 'off', spellcheck: false, ariaLabel: 'Yazıcı IP adresi' });
  rail.replaceChildren(
    h(
      'div',
      { className: 'connect' },
      h('div', { className: 'connect-icon' }, icon('printer', 26)),
      h('h1', { textContent: 'Yazıcını bağla' }),
      h('p', { textContent: 'IP adresini yazıcının ağ özeti sayfasında ya da modem arayüzünde bulabilirsin.' }),
      h(
        'form',
        {
          onsubmit: (e) => {
            e.preventDefault();
            if (input.value.trim()) connect(input.value.trim());
          },
        },
        input,
        h('button', { className: 'btn solid', type: 'submit', textContent: 'Bağlan' }),
      ),
      error ? h('p', { className: 'error', textContent: error }) : null,
    ),
  );
}

function renderLoading() {
  const bar = (height, width = '100%') => h('div', { className: 'skel', style: { height: `${height}px`, width } });
  rail.replaceChildren(h('div', { className: 'rail-scroll' }, bar(22, '40%'), bar(28, '80%'), bar(44), bar(96), bar(84), bar(150)));
}

/** Fetches the snapshot; re-renders only when something actually changed. */
async function refresh(host, quiet = false) {
  try {
    const snap = await invoke('printer_snapshot', { host });
    if (currentHost !== host) return;
    const json = JSON.stringify(snap);
    if (json !== lastSnapshot) {
      lastSnapshot = json;
      renderSnapshot(host, snap);
    }
  } catch (e) {
    if (currentHost !== host || quiet) return;
    console.error(e);
    renderConnect(host, 'Yazıcıya ulaşılamadı. Adresi ve yazıcının açık olduğunu kontrol et.');
  }
}

function connect(host) {
  currentHost = host;
  lastSnapshot = '';
  localStorage.setItem('host', host);
  renderLoading();
  return refresh(host);
}

function confirmJob(host, job) {
  const dialog = $('confirm');
  $('confirmTitle').textContent = `${jobLabel(job)} başlatılsın mı?`;
  dialog.returnValue = 'cancel';
  dialog.onclose = async () => {
    if (dialog.returnValue !== 'ok') return;
    try {
      await invoke('run_maintenance', { host, jobId: job.id });
      toast(`${jobLabel(job)} yazıcıya gönderildi.`);
    } catch (e) {
      toast(`Gönderilemedi: ${e}`, true);
    }
  };
  dialog.showModal();
}

// Keep the status live while the window is visible.
setInterval(() => {
  if (currentHost && !document.hidden) refresh(currentHost, true);
}, 30000);

const savedHost = localStorage.getItem('host');
if (savedHost) connect(savedHost);
else renderConnect();
