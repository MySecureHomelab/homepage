'use strict';

const $$ = (sel) => document.querySelectorAll(sel);

const bytes = (n) => {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let i = 0;
  while (n >= 1024 && i < units.length - 1) { n /= 1024; i++; }
  return `${n.toFixed(i > 2 ? 1 : 0)} ${units[i]}`;
};

const duration = (s) => {
  const d = Math.floor(s / 86400), h = Math.floor(s % 86400 / 3600), m = Math.floor(s % 3600 / 60);
  return d ? `${d}d ${h}h` : h ? `${h}h ${m}m` : `${m}m`;
};

// Icons that fail to load are dropped rather than shown as broken images.
const dropBroken = (img) => { if (img.complete && img.naturalWidth === 0) img.remove(); };
document.addEventListener('error', (e) => { if (e.target.tagName === 'IMG') e.target.remove(); }, true);

function startClock() {
  const time = document.getElementById('time');
  const date = document.getElementById('date');
  if (!time) return;
  const tick = () => {
    const now = new Date();
    time.textContent = now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    date.textContent = now.toLocaleDateString([], { weekday: 'long', day: 'numeric', month: 'long' });
  };
  tick();
  setInterval(tick, 1000);
}

function startSearch() {
  const form = document.getElementById('search');
  if (!form) return;
  form.addEventListener('submit', (e) => {
    e.preventDefault();
    const q = form.q.value.trim();
    if (q) location.href = form.dataset.tpl.replace('{q}', encodeURIComponent(q));
  });
  document.addEventListener('keydown', (e) => {
    const typing = /^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement.tagName);
    if (e.key === '/' && !typing) { e.preventDefault(); form.q.focus(); }
    if (e.key === 'Escape' && document.activeElement === form.q) form.q.blur();
  });
}

function setStat(key, text, ratio) {
  $$(`[data-r="${key}"]`).forEach((el) => { el.textContent = text; });
  if (ratio === undefined) return;
  $$(`[data-bar="${key}"]`).forEach((el) => {
    el.style.width = `${Math.min(100, ratio * 100).toFixed(1)}%`;
    el.className = ratio > 0.9 ? 'crit' : ratio > 0.75 ? 'warn' : '';
  });
}

function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

// Values come from third-party APIs: built with textContent, never innerHTML.
function renderStats(widgets) {
  $$('[data-widget]').forEach((box) => {
    const w = widgets[box.dataset.widget];
    if (!w) return;
    const nodes = w.fields.map(([label, value]) => {
      const block = el('span', 'block');
      block.append(el('b', '', value), el('span', '', label));
      return block;
    });
    if (w.error) nodes.unshift(el('span', 'stats-error', w.error));
    box.replaceChildren(...nodes);
  });
}

function render({ status, widgets, resources: r }) {
  renderStats(widgets);
  $$('[data-check]').forEach((el) => {
    const s = status[el.dataset.check];
    el.classList.toggle('up', !!s?.up);
    el.classList.toggle('down', !!s && !s.up);
    const ms = s?.up && s.ms != null ? `${s.ms} ms` : '';
    el.querySelector('.ms').textContent = ms;
    el.title = !s ? 'Checking…' : s.up ? (ms ? `Up, ${ms}` : 'Up') : 'Down';
  });
  if (r) {
    setStat('cpu', `${r.cpu.toFixed(0)}%`, r.cpu / 100);
    setStat('mem', `${bytes(r.mem_used)} / ${bytes(r.mem_total)}`, r.mem_used / r.mem_total);
    if (r.disk_total) setStat('disk', `${bytes(r.disk_used)} / ${bytes(r.disk_total)}`, r.disk_used / r.disk_total);
    setStat('uptime', duration(r.uptime));
  }
}

// Ctrl/Cmd+K: fuzzy jump to any service or bookmark on the page, or search the web.
function startPalette() {
  const dialog = document.getElementById('palette');
  const input = document.getElementById('palette-q');
  const list = document.getElementById('palette-list');
  const items = [...$$('main a.card')].map((a) => ({
    name: (a.querySelector('.name') ?? a).textContent.trim(),
    detail: a.closest('section')?.querySelector('h2')?.textContent.trim() ?? '',
    href: a.href,
    icon: a.querySelector('img')?.src,
  }));
  let results = [];
  let active = 0;

  // Higher is better; 0 means no match.
  const score = (text, q) => {
    const t = text.toLowerCase();
    if (t.startsWith(q)) return 4;
    if (t.split(/[\s\-_.]+/).some((w) => w.startsWith(q))) return 3;
    if (t.includes(q)) return 2;
    let i = 0;
    for (const c of t) if (c === q[i]) i++;
    return i === q.length ? 1 : 0;
  };

  function select(i) {
    active = i;
    [...list.children].forEach((li, j) => li.setAttribute('aria-selected', String(j === i)));
    input.setAttribute('aria-activedescendant', results.length ? `palette-${i}` : '');
    list.children[i]?.scrollIntoView({ block: 'nearest' });
  }

  function open(r, newTab) {
    if (!r) return;
    dialog.close();
    if (newTab) window.open(r.href, '_blank', 'noopener');
    else location.href = r.href;
  }

  function update() {
    const raw = input.value.trim();
    const q = raw.toLowerCase();
    results = items
      .map((it) => ({ ...it, s: q ? Math.max(score(it.name, q), score(it.detail, q) / 2) : 1 }))
      .filter((it) => it.s > 0)
      .sort((a, b) => b.s - a.s)
      .slice(0, 8);
    if (raw) {
      const href = dialog.dataset.tpl.replace('{q}', encodeURIComponent(raw));
      results.push({ name: `Search the web for “${raw}”`, detail: 'Web', href });
    }
    list.replaceChildren(...results.map((r, i) => {
      const li = el('li');
      li.id = `palette-${i}`;
      li.setAttribute('role', 'option');
      if (r.icon) {
        const img = el('img');
        img.src = r.icon;
        img.alt = '';
        li.append(img);
      }
      li.append(el('span', 'name', r.name), el('span', 'detail', r.detail));
      li.addEventListener('mousemove', () => { if (active !== i) select(i); });
      li.addEventListener('click', (e) => open(r, e.metaKey || e.ctrlKey));
      return li;
    }));
    select(0);
  }

  input.addEventListener('input', update);
  input.addEventListener('keydown', (e) => {
    const n = results.length;
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      if (n) select((active + (e.key === 'ArrowDown' ? 1 : n - 1)) % n);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      open(results[active], e.metaKey || e.ctrlKey);
    }
  });
  document.addEventListener('keydown', (e) => {
    if (e.key.toLowerCase() !== 'k' || !(e.metaKey || e.ctrlKey)) return;
    e.preventDefault();
    if (dialog.open) return dialog.close();
    input.value = '';
    update();
    dialog.showModal();
  });
  // A click on the backdrop lands on the dialog itself; its content covers the rest.
  dialog.addEventListener('click', (e) => { if (e.target === dialog) dialog.close(); });
}

async function refresh() {
  try {
    const res = await fetch('api/state', { cache: 'no-store' });
    if (res.ok) render(await res.json());
  } catch {
    // Server unreachable: keep the last known values on screen.
  }
}

$$('img').forEach(dropBroken);
startClock();
startSearch();
startPalette();
refresh();
setInterval(refresh, 5000);
