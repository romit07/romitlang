/* Romit app runtime — tiny reactive renderer for compiled .rmt apps. */
"use strict";
const S = {};
const C = {};
const A = {};
const V = {};
const R = {
  name: "App", theme: {}, screens: {}, saved: [], timers: [], starts: [], watches: [],
  appearance: () => "auto", init: () => {}, home_screen: null, sink: undefined,
};

/* ---------- values ---------- */
R.list = (v) => Array.isArray(v) ? v : (v === null || v === undefined || v === "") ? [] : [v];
R.num = (v) => { if (typeof v === "number") return v; if (typeof v === "boolean") return v ? 1 : 0; const n = parseFloat(v); return isNaN(n) ? 0 : n; };
R.truthy = (v) => Array.isArray(v) ? v.length > 0 : !!v;
R.isEmpty = (v) => v === null || v === undefined || v === "" || (Array.isArray(v) && v.length === 0) || (typeof v === "object" && !Array.isArray(v) && !(v instanceof Date) && Object.keys(v).length === 0);
R.field = (o, f) => (o === null || o === undefined) ? undefined : o[f];
R.copy = (v) => (v && typeof v === "object" && !(v instanceof Date)) ? JSON.parse(JSON.stringify(v)) : v;
R.same = (a, b) => (a && typeof a === "object") ? a === b || JSON.stringify(a) === JSON.stringify(b) : R.eq(a, b);
R.eq = (a, b) => {
  if (a === b) return true;
  if (a === null || a === undefined || b === null || b === undefined) return R.isEmpty(a) && R.isEmpty(b) && !(typeof a === "number" || typeof b === "number");
  if (typeof a === "number" || typeof b === "number") return R.num(a) === R.num(b) && String(a).trim() !== "" && String(b).trim() !== "";
  if (typeof a === "object" || typeof b === "object") return JSON.stringify(a) === JSON.stringify(b);
  return String(a) === String(b);
};
R.cmp = (a, b) => {
  if (a instanceof Date || b instanceof Date) return new Date(a) - new Date(b);
  if (typeof a === "number" || typeof b === "number") return R.num(a) - R.num(b);
  const x = String(a ?? ""), y = String(b ?? "");
  const nx = parseFloat(x), ny = parseFloat(y);
  if (!isNaN(nx) && !isNaN(ny) && String(nx) === x.trim() && String(ny) === y.trim()) return nx - ny;
  return x.localeCompare(y);
};
R.add = (a, b) => {
  if (Array.isArray(a)) return a.concat(Array.isArray(b) ? b : [b]);
  if (typeof a === "string" || typeof b === "string") return R.show(a) + R.show(b);
  if (a instanceof Date) { const d = new Date(a); d.setDate(d.getDate() + R.num(b)); return d; }
  return R.round(R.num(a) + R.num(b));
};
R.sub = (a, b) => {
  if (Array.isArray(a)) return a.filter((x) => !R.same(x, b));
  if (a instanceof Date && b instanceof Date) return Math.round((a - b) / 86400000);
  return R.round(R.num(a) - R.num(b));
};
R.round = (n) => Math.round(n * 1e10) / 1e10;
R.div = (a, b) => R.num(b) === 0 ? 0 : R.round(R.num(a) / R.num(b));
R.mod = (a, b) => R.num(b) === 0 ? 0 : R.num(a) % R.num(b);
R.inn = (x, coll) => Array.isArray(coll) ? coll.some((y) => R.same(y, x)) : String(coll ?? "").toLowerCase().includes(String(x ?? "").toLowerCase());
R.where = (l, f) => R.list(l).filter((x) => R.truthy(f(x)));
R.sort = (l, field, desc) => {
  const out = R.list(l).slice().sort((a, b) => field ? R.cmp(R.field(a, field), R.field(b, field)) : R.cmp(a, b));
  return desc ? out.reverse() : out;
};
R.shuffle = (l) => { const a = R.list(l).slice(); for (let i = a.length - 1; i > 0; i--) { const j = Math.floor(Math.random() * (i + 1)); [a[i], a[j]] = [a[j], a[i]]; } return a; };
R.rand = (a, b) => { const lo = Math.ceil(Math.min(R.num(a), R.num(b))), hi = Math.floor(Math.max(R.num(a), R.num(b))); return lo + Math.floor(Math.random() * (hi - lo + 1)); };
R.item = (i, l) => { const arr = typeof l === "string" ? [...l] : R.list(l); return arr[R.num(i) - 1] ?? null; };
R.today = () => { const d = new Date(); d.setHours(0, 0, 0, 0); return d; };
R.blank = (v) => Array.isArray(v) ? [] : typeof v === "number" ? 0 : typeof v === "boolean" ? false : (v && typeof v === "object") ? {} : "";
R.of = (f, v) => {
  const l = () => typeof v === "string" && f !== "count" && f !== "length" && f !== "size" ? [...v] : R.list(v);
  const nums = () => R.list(v).map(R.num);
  switch (f) {
    case "__range": { const a = R.num(v[0]), b = R.num(v[1]); const out = []; if (a <= b) for (let i = a; i <= b; i++) out.push(i); else for (let i = a; i >= b; i--) out.push(i); return out; }
    case "count": case "size": case "length": return typeof v === "string" ? [...v].length : (v && typeof v === "object" && !Array.isArray(v)) ? Object.keys(v).length : R.list(v).length;
    case "sum": case "total": return R.round(nums().reduce((a, b) => a + b, 0));
    case "average": { const n = nums(); return n.length ? R.round(n.reduce((a, b) => a + b, 0) / n.length) : 0; }
    case "min": { const s = R.sort(v); return s.length ? s[0] : null; }
    case "max": { const s = R.sort(v); return s.length ? s[s.length - 1] : null; }
    case "first": { const a = l(); return a.length ? a[0] : null; }
    case "last": { const a = l(); return a.length ? a[a.length - 1] : null; }
    case "upper": case "uppercase": return R.show(v).toUpperCase();
    case "lower": case "lowercase": return R.show(v).toLowerCase();
    case "trim": return R.show(v).trim();
    case "round": return Math.round(R.num(v));
    case "floor": return Math.floor(R.num(v));
    case "ceil": return Math.ceil(R.num(v));
    case "abs": return Math.abs(R.num(v));
    case "sqrt": return R.round(Math.sqrt(R.num(v)));
    case "reverse": case "reversed": return typeof v === "string" ? [...v].reverse().join("") : R.list(v).slice().reverse();
    case "sorted": return R.sort(v);
    case "shuffled": return R.shuffle(v);
    case "unique": { const out = []; for (const x of R.list(v)) if (!out.some((y) => R.same(x, y))) out.push(x); return out; }
    case "number": return R.num(v);
    case "text": return R.show(v);
    case "words": return R.show(v).split(/\s+/).filter(Boolean);
    case "lines": return R.show(v).split(/\n/);
    case "keys": return v && typeof v === "object" ? Object.keys(v) : [];
  }
  return null;
};
R.show = (v) => {
  if (v === null || v === undefined) return "";
  if (typeof v === "number") return Number.isInteger(v) ? String(v) : String(R.round(v));
  if (typeof v === "boolean") return v ? "yes" : "no";
  if (v instanceof Date) return v.toLocaleDateString();
  if (Array.isArray(v)) return v.map(R.show).join(", ");
  if (typeof v === "object") return R.show(v.title ?? v.name ?? v.label ?? v.text ?? JSON.stringify(v));
  return String(v);
};
R.fmt = (v, f) => {
  const n = R.num(v);
  switch (f) {
    case "money": case "dollars": return new Intl.NumberFormat(undefined, { style: "currency", currency: "USD" }).format(n);
    case "euros": return new Intl.NumberFormat(undefined, { style: "currency", currency: "EUR" }).format(n);
    case "percent": return new Intl.NumberFormat(undefined, { style: "percent", maximumFractionDigits: 1 }).format(n > 1 ? n / 100 : n);
    case "number": return new Intl.NumberFormat().format(n);
    case "whole": return String(Math.round(n));
    case "decimal": return n.toFixed(2);
    case "date": return new Date(v).toLocaleDateString(undefined, { dateStyle: "medium" });
    case "time": return new Date(v).toLocaleTimeString(undefined, { timeStyle: "short" });
    case "datetime": return new Date(v).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
    case "ago": { const s = Math.round((Date.now() - new Date(v)) / 1000); if (s < 60) return "just now"; if (s < 3600) return Math.floor(s / 60) + " min ago"; if (s < 86400) return Math.floor(s / 3600) + " h ago"; return Math.floor(s / 86400) + " d ago"; }
  }
  return R.show(v);
};
R.opts = (parts) => (parts.length === 1 && Array.isArray(parts[0])) ? parts[0] : parts;
R.each = (l, f) => { const arr = (typeof l === "number") ? R.of("__range", [1, l]) : R.list(l); return arr.map((x, i) => f(x, i + 1)); };

/* ---------- virtual DOM ---------- */
const flat = (kids, out = []) => { for (const k of kids) { if (k === null || k === undefined || k === false) continue; if (Array.isArray(k)) flat(k, out); else if (typeof k === "object" && k.t) out.push(k); else out.push({ t: "#", v: String(k) }); } return out; };
const h = (t, a, ...kids) => ({ t, a: a || {}, c: flat(kids) });
R.h = h;
const EVENT = /^on[A-Z]/;
function setAttrs(el, a, old) {
  for (const k in old) if (!(k in a)) {
    if (EVENT.test(k)) { el._h && delete el._h[k]; }
    else if (k === "style") el.removeAttribute("style");
    else if (k !== "key") el.removeAttribute(k === "className" ? "class" : k);
  }
  for (const k in a) {
    const v = a[k];
    if (k === "key") continue;
    if (EVENT.test(k)) {
      el._h = el._h || {};
      if (!el._h[k]) { const name = k.slice(2).toLowerCase(); el.addEventListener(name, (e) => el._h[k] && el._h[k](e)); }
      el._h[k] = v;
    } else if (k === "style") {
      const s = typeof v === "string" ? v : Object.entries(v).filter(([, x]) => x !== null && x !== undefined && x !== "").map(([p, x]) => p + ":" + x).join(";");
      if (el.getAttribute("style") !== s) el.setAttribute("style", s);
    } else if (k === "value") {
      if (el.value !== String(v ?? "")) el.value = v ?? "";
    } else if (k === "checked" || k === "disabled" || k === "open" || k === "autofocus") {
      el[k] = !!v;
    } else if (k === "html") {
      if (el.innerHTML !== v) el.innerHTML = v;
    } else if (v === false || v === null || v === undefined) {
      el.removeAttribute(k === "className" ? "class" : k);
    } else {
      const name = k === "className" ? "class" : k;
      if (el.getAttribute(name) !== String(v)) el.setAttribute(name, v === true ? "" : v);
    }
  }
}
function mount(v) {
  if (v.t === "#") { const n = document.createTextNode(v.v); n._v = v; return n; }
  const el = v.a.svg ? document.createElementNS("http://www.w3.org/2000/svg", v.t) : document.createElement(v.t);
  setAttrs(el, v.a, {});
  if (!v.a.html) for (const c of v.c) el.appendChild(mount(c));
  el._v = v;
  if (v.a.autofocus) setTimeout(() => el.focus && el.focus(), 30);
  return el;
}
function patch(el, v) {
  const o = el._v;
  if (!o || o.t !== v.t || (o.a && v.a && o.a.key !== v.a.key)) { const n = mount(v); el.replaceWith(n); return n; }
  if (v.t === "#") { if (el.nodeValue !== v.v) el.nodeValue = v.v; el._v = v; return el; }
  setAttrs(el, v.a, o.a);
  if (!v.a.html) patchKids(el, v.c);
  el._v = v;
  return el;
}
function patchKids(el, kids) {
  const doms = Array.from(el.childNodes);
  for (let i = 0; i < kids.length; i++) {
    if (i < doms.length) patch(doms[i], kids[i]); else el.appendChild(mount(kids[i]));
  }
  for (let i = kids.length; i < doms.length; i++) doms[i].remove();
}

/* ---------- icons (SF Symbols names → glyphs) ---------- */
const ICONS = {
  house: "⌂", home: "⌂", gear: "⚙", gearshape: "⚙", settings: "⚙", star: "☆", "star.fill": "★", heart: "♡", "heart.fill": "♥",
  plus: "+", add: "+", minus: "−", "plus.circle": "⊕", "plus.circle.fill": "⊕", "minus.circle": "⊖", xmark: "✕", close: "✕", "xmark.circle": "⊗", checkmark: "✓", check: "✓",
  "checkmark.circle": "✓", "checkmark.circle.fill": "✓", circle: "○", "circle.fill": "●", trash: "🗑", delete: "🗑", pencil: "✎", edit: "✎", "square.and.pencil": "✎",
  magnifyingglass: "⌕", search: "⌕", person: "👤", "person.fill": "👤", user: "👤", "person.2": "👥", people: "👥", bell: "🔔", "bell.fill": "🔔",
  envelope: "✉", mail: "✉", phone: "✆", message: "💬", chat: "💬", bubble: "💬", calendar: "📅", clock: "🕒", timer: "⏱", alarm: "⏰",
  cart: "🛒", bag: "👜", creditcard: "💳", book: "📖", bookmark: "🔖", tag: "🏷", flag: "⚑", folder: "📁", doc: "📄", "doc.text": "📄", paperclip: "📎",
  photo: "🖼", camera: "📷", video: "🎥", mic: "🎤", music: "♪", "music.note": "♪", play: "▶", "play.fill": "▶", pause: "⏸", stop: "⏹", forward: "⏩", backward: "⏪",
  "arrow.right": "→", "arrow.left": "←", "arrow.up": "↑", "arrow.down": "↓", "chevron.right": "›", "chevron.left": "‹", "chevron.down": "⌄", "chevron.up": "⌃",
  "arrow.clockwise": "↻", refresh: "↻", "square.and.arrow.up": "⇪", share: "⇪", link: "🔗", globe: "🌐", map: "🗺", location: "📍", pin: "📍", "mappin": "📍",
  sun: "☀", "sun.max": "☀", moon: "☾", "moon.fill": "☾", cloud: "☁", bolt: "⚡", flame: "🔥", fire: "🔥", drop: "💧", leaf: "🍃", snowflake: "❄",
  lock: "🔒", "lock.open": "🔓", key: "🔑", shield: "🛡", eye: "👁", "eye.slash": "🙈", info: "ⓘ", "info.circle": "ⓘ", questionmark: "?", "questionmark.circle": "?",
  exclamationmark: "!", "exclamationmark.triangle": "⚠", warning: "⚠", tray: "📥", inbox: "📥", archivebox: "🗄", list: "☰", "list.bullet": "☰", menu: "☰",
  "line.3.horizontal": "☰", ellipsis: "…", more: "…", grid: "▦", "square.grid.2x2": "▦", chart: "📊", "chart.bar": "📊", gift: "🎁", cup: "☕", fork: "🍴",
  car: "🚗", airplane: "✈", bicycle: "🚲", figure: "🚶", "figure.walk": "🚶", sparkles: "✨", wand: "🪄", lightbulb: "💡", hand: "✋", "hand.thumbsup": "👍",
  "face.smiling": "☺", smile: "☺", crown: "♛", trophy: "🏆", target: "◎", dollarsign: "$", money: "$", percent: "%", number: "#", textformat: "Aa",
};
R.icon = (name) => { const n = R.show(name); return ICONS[n] ?? ICONS[n.replace(/\.fill$/, "")] ?? n; };

/* ---------- elements ---------- */
const fire = (fn, e) => { if (fn) { if (e) e.stopPropagation(); fn(); } };
const label = (p) => p.a.length ? p.a.map(R.show).join("") : (p.label !== undefined ? R.show(p.label) : "");
const bindGet = (p) => p.bind ? p.bind.get() : undefined;
const bindSet = (p, v) => { if (p.bind) { p.bind.set(v); R.commit(); if (p.on && p.on.change) p.on.change(); } };
let uid = 0;
R.el = (kind, p, kids) => {
  kids = flat([kids]);
  const on = p.on || {};
  const text = (cls) => h("p", { class: "r-text " + cls }, label(p));
  let node;
  switch (kind) {
    case "column": case "row": case "stack": case "group": case "wrap":
      node = h("div", { class: "r-" + kind }, kids); break;
    case "grid": {
      const cols = p.columns ?? p.a[0] ?? 2;
      node = h("div", { class: "r-grid", style: { "grid-template-columns": typeof cols === "number" ? `repeat(${cols}, minmax(0, 1fr))` : `repeat(auto-fill, minmax(${R.num(p.min ?? 140)}px, 1fr))` } }, kids); break;
    }
    case "scroll": node = h("div", { class: "r-scroll" + (p.a.some((x) => R.show(x) === "horizontal") ? " r-scroll-x" : "") }, kids); break;
    case "spacer": node = h("div", { class: "r-spacer", style: p.a.length ? { "flex": "0 0 " + R.num(p.a[0]) + "px" } : {} }); break;
    case "divider": node = h("hr", { class: "r-divider" }); break;
    case "list": node = h("div", { class: "r-list", role: "list" }, kids.map((k) => listRow(k))); break;
    case "form": node = h("form", { class: "r-form", onSubmit: (e) => { e.preventDefault(); fire(on.submit); } }, kids); break;
    case "section": {
      const header = p.a.length ? label(p) : (p.header !== undefined ? R.show(p.header) : "");
      node = h("section", { class: "r-section" }, header ? h("h3", { class: "r-section-header" }, header) : null,
        h("div", { class: "r-section-body" }, kids.map((k) => listRow(k))),
        p.footer !== undefined ? h("p", { class: "r-section-footer" }, R.show(p.footer)) : null); break;
    }
    case "box": node = h("div", { class: "r-box" }, p.a.length ? h("div", { class: "r-box-title" }, label(p)) : null, kids); break;
    case "card": node = h("div", { class: "r-card" }, p.a[0] !== undefined ? h("div", { class: "r-card-title" }, R.show(p.a[0])) : null, p.a[1] !== undefined ? h("div", { class: "r-card-sub" }, R.show(p.a[1])) : null, kids); break;
    case "disclosure": node = h("details", { class: "r-disclosure", open: p.bind ? R.truthy(bindGet(p)) : undefined, onToggle: (e) => { if (p.bind && R.truthy(p.bind.get()) !== e.target.open) bindSet(p, e.target.open); } }, h("summary", {}, label(p)), h("div", { class: "r-disclosure-body" }, kids)); break;
    case "text": node = kids.length ? h("p", { class: "r-text" }, label(p), kids) : text(""); break;
    case "title": node = h("h1", { class: "r-text r-title" }, label(p)); break;
    case "heading": node = h("h2", { class: "r-text r-heading" }, label(p)); break;
    case "subheading": node = h("h3", { class: "r-text r-subheading" }, label(p)); break;
    case "caption": node = text("r-caption"); break;
    case "label": node = h("span", { class: "r-label" }, p.icon !== undefined ? h("span", { class: "r-icon", "aria-hidden": "true" }, R.icon(p.icon)) : null, h("span", {}, label(p))); break;
    case "icon": node = h("span", { class: "r-icon", role: "img", "aria-label": R.show(p.a[0]) }, R.icon(p.a[0])); break;
    case "image": node = h("img", { class: "r-image", src: R.show(p.a[0]), alt: p.a[1] !== undefined ? R.show(p.a[1]) : "", loading: "lazy" }); break;
    case "video": node = h("video", { class: "r-video", src: R.show(p.a[0]), controls: true, playsinline: true }); break;
    case "circle": case "rectangle": case "capsule": case "rounded": case "ellipse":
      node = h("div", { class: "r-shape r-" + kind }); break;
    case "button": {
      const go = p.nav ? () => R.go(p.nav.screen, p.nav.args) : null;
      const content = kids.length ? kids : [p.icon !== undefined ? h("span", { class: "r-icon" }, R.icon(p.icon)) : null, p.a.length ? h("span", {}, label(p)) : null];
      node = h("button", { type: "button", class: "r-button", onClick: (e) => { e.stopPropagation(); if (on.tap) fire(on.tap); if (go) go(); } }, content); break;
    }
    case "link": {
      const content = kids.length ? kids : [p.icon !== undefined ? h("span", { class: "r-icon" }, R.icon(p.icon)) : null, h("span", {}, label(p) || R.show(p.to))];
      if (p.nav) node = h("a", { href: "#", class: "r-link r-navlink", onClick: (e) => { e.preventDefault(); e.stopPropagation(); fire(on.tap); R.go(p.nav.screen, p.nav.args); } }, content, h("span", { class: "r-chevron", "aria-hidden": "true" }, "›"));
      else node = h("a", { href: R.show(p.to), class: "r-link", target: "_blank", rel: "noopener", onClick: (e) => { e.stopPropagation(); fire(on.tap); } }, content);
      break;
    }
    case "toggle": {
      const id = "r" + (++uid);
      node = h("label", { class: "r-toggle" + (label(p) ? "" : " r-toggle-bare"), for: id }, label(p) ? h("span", { class: "r-toggle-label" }, label(p)) : null,
        h("input", { id, type: "checkbox", role: "switch", checked: R.truthy(bindGet(p)), onChange: (e) => { bindSet(p, e.target.checked); fire(on.tap); } }), h("span", { class: "r-switch", "aria-hidden": "true" })); break;
    }
    case "field": case "secure": case "search": {
      const kb = p.keyboard;
      node = h("label", { class: "r-field" + (kind === "search" ? " r-search" : "") }, kind === "search" ? h("span", { class: "r-icon", "aria-hidden": "true" }, "⌕") : null,
        h("input", { type: kind === "secure" ? "password" : kind === "search" ? "search" : (kb || "text"), placeholder: label(p) || (p.placeholder !== undefined ? R.show(p.placeholder) : (kind === "search" ? "Search" : "")),
          "aria-label": label(p) || "text", value: bindGet(p) ?? "",
          onInput: (e) => { const cur = bindGet(p); bindSet(p, typeof cur === "number" ? R.num(e.target.value) : e.target.value); },
          onKeydown: (e) => { if (e.key === "Enter") { e.preventDefault(); fire(on.submit); } } })); break;
    }
    case "editor": node = h("textarea", { class: "r-editor", placeholder: label(p), value: bindGet(p) ?? "", onInput: (e) => bindSet(p, e.target.value) }); break;
    case "slider": {
      const v = R.num(bindGet(p));
      node = h("label", { class: "r-slider" }, p.a.length ? h("span", { class: "r-slider-label" }, label(p)) : null,
        h("input", { type: "range", min: R.num(p.from ?? 0), max: R.num(p.to ?? 100), step: p.step ?? "any", value: v, "aria-label": label(p) || "value", onInput: (e) => bindSet(p, R.num(e.target.value)) }),
        h("span", { class: "r-slider-value" }, R.show(Math.round(v * 100) / 100))); break;
    }
    case "stepper": {
      const v = R.num(bindGet(p)), step = R.num(p.step ?? 1), lo = p.from !== undefined ? R.num(p.from) : -Infinity, hi = p.to !== undefined ? R.num(p.to) : Infinity;
      node = h("div", { class: "r-stepper" }, h("span", { class: "r-stepper-label" }, (label(p) ? label(p) + ": " : "") + R.show(v)),
        h("div", { class: "r-stepper-buttons" },
          h("button", { type: "button", "aria-label": "decrease", disabled: v - step < lo, onClick: () => bindSet(p, R.round(Math.max(lo, v - step))) }, "−"),
          h("button", { type: "button", "aria-label": "increase", disabled: v + step > hi, onClick: () => bindSet(p, R.round(Math.min(hi, v + step))) }, "+"))); break;
    }
    case "picker": {
      const opts = R.list(p.options), cur = bindGet(p), style = p.__style || (opts.length <= 4 ? "segmented" : "menu");
      if (style === "segmented") node = h("div", { class: "r-picker-wrap" }, label(p) ? h("span", { class: "r-picker-label" }, label(p)) : null, h("div", { class: "r-segmented", role: "radiogroup" }, opts.map((o) => h("button", { type: "button", role: "radio", "aria-checked": R.same(o, cur) ? "true" : "false", class: R.same(o, cur) ? "on" : "", onClick: () => bindSet(p, o) }, R.show(o)))));
      else node = h("label", { class: "r-picker" }, h("span", {}, label(p)), h("select", { onChange: (e) => bindSet(p, opts[+e.target.value]) }, opts.map((o, i) => h("option", { value: i, selected: R.same(o, cur) ? true : false }, R.show(o))))); break;
    }
    case "date": {
      const v = bindGet(p); const d = v ? new Date(v) : null;
      const iso = d && !isNaN(d) ? new Date(d.getTime() - d.getTimezoneOffset() * 60000).toISOString().slice(0, 10) : "";
      node = h("label", { class: "r-picker" }, h("span", {}, label(p)), h("input", { type: "date", value: iso, onInput: (e) => bindSet(p, e.target.value ? new Date(e.target.value + "T00:00") : null) })); break;
    }
    case "colorpicker": node = h("label", { class: "r-picker" }, h("span", {}, label(p)), h("input", { type: "color", value: R.show(bindGet(p)) || "#007aff", onInput: (e) => bindSet(p, e.target.value) })); break;
    case "menu": {
      const id = "m" + (++uid);
      node = h("details", { class: "r-menu" }, h("summary", { class: "r-button" }, p.icon !== undefined ? h("span", { class: "r-icon" }, R.icon(p.icon)) : null, label(p)), h("div", { class: "r-menu-items", id, onClick: (e) => { const d = e.currentTarget.parentElement; setTimeout(() => d.removeAttribute("open"), 0); } }, kids)); break;
    }
    case "share": node = h("button", { type: "button", class: "r-button", onClick: () => R.share(p.a[0] ?? p.value) }, h("span", { class: "r-icon" }, "⇪"), h("span", {}, p.a[1] !== undefined ? R.show(p.a[1]) : "Share")); break;
    case "progress": {
      if (!p.a.length && p.value === undefined) { node = h("div", { class: "r-spinner", role: "progressbar", "aria-label": "Loading" }); break; }
      const val = R.num(p.a[0] ?? p.value), total = p.of !== undefined ? R.num(p.of) : (val > 1 ? 100 : 1);
      const pct = total ? Math.max(0, Math.min(100, (val / total) * 100)) : 0;
      node = h("div", { class: "r-progress", role: "progressbar", "aria-valuenow": Math.round(pct), "aria-valuemin": 0, "aria-valuemax": 100 }, h("div", { class: "r-progress-bar", style: { width: pct + "%" } })); break;
    }
    case "spinner": node = h("div", { class: "r-spinner-wrap" }, h("div", { class: "r-spinner", role: "progressbar", "aria-label": "Loading" }), p.a.length ? h("span", {}, label(p)) : null); break;
    case "gauge": {
      const val = R.num(p.a[0] ?? p.value), lo = R.num(p.from ?? 0), hi = R.num(p.to ?? (p.of ?? 100));
      const pct = hi > lo ? Math.max(0, Math.min(1, (val - lo) / (hi - lo))) : 0;
      node = h("div", { class: "r-gauge", style: { "--p": pct }, role: "meter", "aria-valuenow": val }, h("div", { class: "r-gauge-inner" }, R.show(val)), p.a[1] !== undefined ? h("div", { class: "r-gauge-label" }, R.show(p.a[1])) : null); break;
    }
    case "empty": node = h("div", { class: "r-empty" }, h("div", { class: "r-empty-icon" }, R.icon(p.icon ?? "tray")), h("div", { class: "r-empty-title" }, label(p) || "Nothing here"), p.message !== undefined ? h("div", { class: "r-empty-message" }, R.show(p.message)) : null, kids); break;
    default: node = h("div", {}, kids);
  }
  if (on.tap && !["button", "link", "toggle", "menu", "share"].includes(kind)) {
    node.a.onClick = (e) => fire(on.tap, e);
    node.a.class = (node.a.class || "") + " r-tappable";
    if (!node.a.role) node.a.role = "button";
    node.a.tabindex = 0;
    node.a.onKeydown = (e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); fire(on.tap, e); } };
  }
  if (on.doubletap) node.a.onDblclick = (e) => fire(on.doubletap, e);
  if (on.hover) node.a.onMouseenter = () => fire(on.hover);
  if (on.longpress) {
    let t;
    node.a.onPointerdown = () => { t = setTimeout(() => fire(on.longpress), 550); };
    node.a.onPointerup = node.a.onPointerleave = () => clearTimeout(t);
    node.a.onContextmenu = (e) => e.preventDefault();
  }
  if (on.delete) node.__delete = on.delete;
  node.__kind = kind;
  node.__p = p;
  return node;
};
function listRow(k) {
  if (!k || !k.t) return k;
  if (k.__kind === "section") return k;
  const del = k.__delete;
  return h("div", { class: "r-row-item", role: "listitem" }, h("div", { class: "r-row-content" }, k),
    del ? h("button", { type: "button", class: "r-row-delete", "aria-label": "Delete", onClick: (e) => fire(del, e) }, "Delete") : null);
}
R.comp = (nodes, mods, on) => {
  const list = flat([nodes]);
  if (!mods.filter(Boolean).length && !Object.keys(on).length && list.length === 1) { if (on.delete) list[0].__delete = on.delete; return list[0]; }
  const n = R.el("group", { a: [], on }, list);
  return R.mod(n, mods);
};

/* ---------- modifiers ---------- */
const px = (v) => typeof v === "number" ? v + "px" : R.show(v);
const FONTS = { title: "var(--fs-title)", largetitle: "var(--fs-title)", big: "1.6rem", huge: "3rem", heading: "1.35rem", headline: "1.1rem", subheading: "1.1rem", body: "1rem", callout: "0.95rem", caption: "0.8rem", small: "0.85rem", footnote: "0.8rem" };
const WEIGHTS = { thin: 200, light: 300, regular: 400, normal: 400, medium: 500, semibold: 600, bold: 700, heavy: 800, black: 900 };
R.mod = (node, mods) => {
  if (!node || !node.a) return node;
  const st = {}; const cls = [];
  const p = node.__p || {};
  for (const m of mods) {
    if (!m) continue;
    const [name, v, w] = m;
    switch (name) {
      case "padding": {
        if (v === undefined) { st.padding = "var(--space)"; break; }
        const edge = typeof v === "string" && isNaN(parseFloat(v)) ? v : null;
        const amt = edge ? (w !== undefined ? px(w) : "var(--space)") : px(v);
        if (!edge) st.padding = amt;
        else if (edge === "horizontal") { st["padding-left"] = amt; st["padding-right"] = amt; }
        else if (edge === "vertical") { st["padding-top"] = amt; st["padding-bottom"] = amt; }
        else st["padding-" + ({ leading: "left", trailing: "right" }[edge] || edge)] = amt;
        break;
      }
      case "gap": st.gap = v === undefined ? "var(--space)" : px(v); break;
      case "width": if (R.show(v) === "full") st.width = "100%"; else { st.width = px(v); st["flex-shrink"] = 0; } break;
      case "height": st.height = px(v); st["flex-shrink"] = 0; break;
      case "maxwidth": st["max-width"] = px(v); st.width = "100%"; st["margin-inline"] = "auto"; break;
      case "minheight": st["min-height"] = px(v); break;
      case "size": {
        if (["circle", "rectangle", "capsule", "rounded", "ellipse", "image", "video"].includes(node.__kind)) { st.width = px(v); st.height = px(v); st["flex-shrink"] = 0; }
        else st["font-size"] = typeof v === "number" ? v + "px" : (FONTS[R.show(v).toLowerCase()] || R.show(v));
        break;
      }
      case "stretch": st["align-self"] = "stretch"; st.width = "100%"; break;
      case "grow": st.flex = "1 1 0"; break;
      case "color": st.color = R.show(v); st["--accent-ink"] = R.show(v); break;
      case "background": st.background = R.show(v); if (!["circle", "rectangle", "capsule", "rounded", "ellipse"].includes(node.__kind)) cls.push("r-has-bg"); break;
      case "gradient": st.background = `linear-gradient(135deg, ${R.show(v)}, ${R.show(w ?? "transparent")})`; cls.push("r-has-bg"); break;
      case "tint": st["--accent"] = R.show(v); break;
      case "fill": st.background = R.show(v); break;
      case "stroke": st.border = "2px solid " + R.show(v); break;
      case "corner": st["border-radius"] = v === undefined ? "var(--corner)" : px(v); st.overflow = "hidden"; break;
      case "border": st.border = `${w !== undefined ? px(w) : "1px"} solid ${R.show(v)}`; break;
      case "shadow": { const r = v === undefined ? 12 : R.num(v); st["box-shadow"] = `0 ${Math.round(r / 3)}px ${r}px rgba(0,0,0,.14)`; break; }
      case "opacity": st.opacity = R.num(v) > 1 ? R.num(v) / 100 : R.num(v); break;
      case "blur": st.filter = `blur(${v === undefined ? 4 : R.num(v)}px)`; break;
      case "rotate": st.rotate = R.num(v) + "deg"; break;
      case "scale": st.scale = String(R.num(v)); break;
      case "offset": st.translate = `${px(v)} ${px(w ?? 0)}`; break;
      case "z": st["z-index"] = R.num(v); st.position = "relative"; break;
      case "hidden": cls.push("r-hidden"); break;
      case "disabled": node.a.disabled = true; cls.push("r-disabled"); node.a["aria-disabled"] = "true"; break;
      case "bold": st["font-weight"] = 700; break;
      case "italic": st["font-style"] = "italic"; break;
      case "underline": case "strike": {
        const deco = name === "strike" ? "line-through" : "underline";
        if (name === "strike") cls.push("r-struck");
        // links: decorate the words, not the chevron
        if (node.__kind === "link") for (const c of node.c) { if (c.a && !String(c.a.class || "").includes("r-chevron")) c.a.style = Object.assign({}, c.a.style || {}, { "text-decoration": deco }); }
        else st["text-decoration"] = deco;
        break;
      }
      case "mono": st["font-family"] = "var(--mono)"; break;
      case "serif": st["font-family"] = "ui-serif, Georgia, serif"; break;
      case "uppercase": st["text-transform"] = "uppercase"; st["letter-spacing"] = ".04em"; break;
      case "lowercase": st["text-transform"] = "lowercase"; break;
      case "font": {
        const f = R.show(v).toLowerCase();
        if (FONTS[f]) st["font-size"] = FONTS[f];
        else if (f === "mono" || f === "monospaced") st["font-family"] = "var(--mono)";
        else if (f === "serif") st["font-family"] = "ui-serif, Georgia, serif";
        else if (f === "rounded") st["font-family"] = "ui-rounded, 'SF Pro Rounded', system-ui, sans-serif";
        else st["font-family"] = `${JSON.stringify(R.show(v))}, var(--font)`;
        if (["title", "largetitle", "heading", "headline", "subheading"].includes(f)) st["font-weight"] = 700;
        break;
      }
      case "weight": st["font-weight"] = WEIGHTS[R.show(v).toLowerCase()] ?? R.num(v); break;
      case "lines": cls.push("r-clamp"); st["-webkit-line-clamp"] = R.num(v); break;
      case "align": case "center": case "leading": case "trailing": case "top": case "bottom": {
        const a = name === "align" ? R.show(v) : name;
        cls.push("r-align-" + a); break;
      }
      case "selectable": st["user-select"] = "text"; break;
      case "help": node.a.title = R.show(v); break;
      case "describe": node.a["aria-label"] = R.show(v); break;
      case "clip": st.overflow = "hidden"; if (R.show(v) === "circle") st["border-radius"] = "50%"; else if (v !== undefined) st["border-radius"] = "var(--corner)"; break;
      case "aspect": st["object-fit"] = R.show(v) === "fill" ? "cover" : "contain"; break;
      case "transition": cls.push("r-tr-" + (v === undefined ? "fade" : R.show(v))); break;
      case "animated": cls.push("r-animated"); break;
      case "badge": { if (!R.isEmpty(v) && v !== 0) node.c.push(h("span", { class: "r-badge" }, R.show(v))); break; }
      case "focused": node.a.autofocus = true; if (node.c[0] && node.c[0].a && node.t === "label") for (const c of node.c) if (c.t === "input") c.a.autofocus = true; break;
      case "style": {
        const s = R.show(v).toLowerCase();
        if (node.__kind === "picker") { /* handled at build time via __style */ }
        cls.push("r-style-" + s); break;
      }
      case "role": cls.push("r-role-" + R.show(v).toLowerCase()); break;
      case "keyboard": {
        const map = { number: "number", decimal: "number", email: "email", phone: "tel", url: "url", text: "text" };
        for (const c of node.c) if (c.t === "input") { c.a.type = map[R.show(v)] || "text"; if (R.show(v) === "decimal") c.a.step = "any"; c.a.inputmode = R.show(v) === "phone" ? "tel" : R.show(v); }
        break;
      }
      case "sticky": st.position = "sticky"; st.top = 0; st["z-index"] = 5; break;
    }
  }
  if (node.__kind === "picker") {
    const sm = mods.find((m) => m && m[0] === "style");
    if (sm && R.show(sm[1]) !== (node.__p.__style || "")) { node.__p.__style = R.show(sm[1]) === "wheel" ? "menu" : R.show(sm[1]); const n2 = R.el("picker", node.__p, []); return R.mod(n2, mods.filter((m) => !m || m[0] !== "style")); }
  }
  if (Object.keys(st).length) {
    const old = node.a.style && typeof node.a.style === "object" ? node.a.style : {};
    node.a.style = Object.assign({}, old, st);
  }
  if (cls.length) node.a.class = ((node.a.class || "") + " " + cls.join(" ")).trim();
  return node;
};

/* ---------- navigation, sheets, dialogs ---------- */
const nav = { tab: 0, stacks: [], sheet: null, dialog: null, toasts: [] };
R.nav = nav;
const tabs = () => Object.values(R.screens).filter((s) => s.tab !== null);
const stack = () => nav.stacks[nav.tab] || (nav.stacks[nav.tab] = []);
R.go = (name, args) => { if (nav.sheet) nav.sheet = null; stack().push({ name, args: args || [] }); R.commit(); window.scrollTo && window.scrollTo(0, 0); };
R.back = () => { if (nav.sheet) nav.sheet = null; else if (stack().length > 1) stack().pop(); R.commit(); };
R.home = () => { nav.sheet = null; nav.stacks[nav.tab] = stack().slice(0, 1); R.commit(); };
R.sheet = (name, args) => { nav.sheet = { name, args: args || [] }; R.commit(); };
R.close = () => { if (nav.sheet) nav.sheet = null; else if (stack().length > 1) stack().pop(); R.commit(); };
R.alert = (title, message) => { nav.dialog = { title: R.show(title), message: message === null ? null : R.show(message), buttons: [{ label: "OK", run: null, primary: true }] }; R.commit(); };
R.ask = (title, message, yes) => { nav.dialog = { title: R.show(title), message: message === null ? null : R.show(message), buttons: [{ label: "Cancel", run: null }, { label: "OK", run: yes, primary: true, danger: true }] }; R.commit(); };
R.toast = (msg) => { const t = { id: ++uid, text: R.show(msg) }; nav.toasts.push(t); R.commit(); setTimeout(() => { nav.toasts = nav.toasts.filter((x) => x !== t); R.commit(); }, 2200); };
R.wait = (ms) => { R.commit(); return new Promise((r) => setTimeout(r, ms)); };
R.open = (url) => { const u = R.show(url); window.open(/^[a-z]+:/i.test(u) ? u : "https://" + u, "_blank", "noopener"); };
R.copyText = (v) => { const t = R.show(v); (navigator.clipboard ? navigator.clipboard.writeText(t) : Promise.reject()).then(() => R.toast("Copied"), () => R.toast("Couldn't copy")); };
R.share = (v) => { const t = R.show(v); if (navigator.share) navigator.share({ text: t }).catch(() => {}); else R.copyText(t); };
R.say = (v) => console.log("[romit]", R.show(v));
R.vibrate = () => { if (navigator.vibrate) navigator.vibrate(12); };
R.animate = () => { document.documentElement.classList.add("r-animating"); clearTimeout(R._anim); R._anim = setTimeout(() => document.documentElement.classList.remove("r-animating"), 450); };
R.load = async (url) => {
  R.commit();
  try {
    const res = await fetch(R.show(url));
    if (!res.ok) throw new Error(res.status + " " + res.statusText);
    const type = res.headers.get("content-type") || "";
    return type.includes("json") ? await res.json() : await res.text();
  } catch (err) { R.toast("Couldn't load: " + err.message); throw err; }
};

/* ---------- running actions ---------- */
let depth = 0;
R.act = (fn) => async () => {
  depth++;
  try { await fn(); }
  catch (err) { if (!(err instanceof TypeError && /fetch/i.test(err.message))) console.error("[romit]", err); }
  finally { depth--; R.commit(); }
};

/* ---------- storage ---------- */
const key = (n) => "romit:" + R.name + ":" + n;
const revive = (v) => typeof v === "string" && /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d/.test(v) ? new Date(v) : Array.isArray(v) ? v.map(revive) : (v && typeof v === "object") ? Object.fromEntries(Object.entries(v).map(([k, x]) => [k, revive(x)])) : v;
function loadSaved() { for (const n of R.saved) { try { const raw = localStorage.getItem(key(n)); if (raw !== null) S[n] = revive(JSON.parse(raw)); } catch (e) { /* storage unavailable */ } } }
function save() { for (const n of R.saved) { try { localStorage.setItem(key(n), JSON.stringify(S[n])); } catch (e) { /* storage unavailable */ } } }

/* ---------- drawing ---------- */
let queued = false, drawing = false;
R.commit = () => { save(); checkWatches(); if (!queued) { queued = true; queueMicrotask(draw); } };
function checkWatches() {
  const all = R.watches.map((w) => [w, () => w.get()]);
  for (const s of activeScreens()) for (const w of (R.screens[s.name].watches || [])) all.push([w, () => w.get(...s.args), s]);
  for (const [w, get, s] of all) {
    let v; try { v = JSON.stringify(get()); } catch (e) { continue; }
    const k = s ? "__last_" + s.name : "__last";
    if (w[k] === undefined) { w[k] = v; continue; }
    if (w[k] !== v) { w[k] = v; if (depth < 20) s ? w.run(...s.args) : w.run(); }
  }
}
function activeScreens() { const out = []; const st = stack(); if (st.length) out.push(st[st.length - 1]); if (nav.sheet) out.push(nav.sheet); return out; }
let lastActive = new Set();
function screenView(entry, isSheet) {
  const sc = R.screens[entry.name];
  if (!sc) return h("div", {}, "missing screen " + entry.name);
  const args = entry.args;
  const st = stack();
  const canBack = !isSheet && st.length > 1;
  const toolbar = sc.toolbar(...args);
  const title = R.show(sc.title(...args));
  return h("div", { class: "r-screen" + (isSheet ? " r-screen-sheet" : "") + (canBack || isSheet ? " r-inline" : " r-root"), key: entry.name + (isSheet ? ":sheet" : ":" + st.length) },
    h("header", { class: "r-navbar" },
      h("div", { class: "r-navbar-lead" }, canBack ? h("button", { type: "button", class: "r-back", onClick: () => R.back() }, "‹ ", R.show(R.screens[st[st.length - 2].name].title(...st[st.length - 2].args))) : null,
        isSheet ? h("button", { type: "button", class: "r-back", onClick: () => R.close() }, "Close") : null),
      h("div", { class: "r-navbar-title" }, title),
      h("div", { class: "r-navbar-trail" }, toolbar)),
    h("main", { class: "r-content" }, title ? h("h1", { class: "r-large-title" }, title) : null, sc.body(...args)));
}
function draw() {
  queued = false;
  if (drawing) return;
  drawing = true;
  try {
    const root = document.getElementById("romit-app");
    const mode = R.show(R.appearance());
    document.documentElement.dataset.theme = mode === "dark" ? "dark" : mode === "light" ? "light" : "";
    const ts = tabs();
    const st = stack();
    if (!st.length) st.push({ name: ts.length ? ts[nav.tab].name : R.home_screen, args: [] });
    const top = st[st.length - 1];
    const kids = [screenView(top, false)];
    if (ts.length > 1) kids.push(h("nav", { class: "r-tabbar" }, ts.map((s, i) => h("button", { type: "button", class: "r-tab" + (i === nav.tab ? " on" : ""), "aria-current": i === nav.tab ? "page" : false, onClick: () => { if (nav.tab === i) R.home(); else { nav.tab = i; R.commit(); } } }, h("span", { class: "r-icon" }, R.icon(s.tab || "circle")), h("span", { class: "r-tab-label" }, R.show(s.title()))))));
    if (nav.sheet) kids.push(h("div", { class: "r-sheet-backdrop", onClick: (e) => { if (e.target === e.currentTarget) R.close(); } }, h("div", { class: "r-sheet", role: "dialog", "aria-modal": "true" }, screenView(nav.sheet, true))));
    if (nav.dialog) {
      const d = nav.dialog;
      kids.push(h("div", { class: "r-dialog-backdrop" }, h("div", { class: "r-dialog", role: "alertdialog", "aria-modal": "true" },
        h("div", { class: "r-dialog-title" }, d.title), d.message ? h("div", { class: "r-dialog-message" }, d.message) : null,
        h("div", { class: "r-dialog-buttons" }, d.buttons.map((b) => h("button", { type: "button", class: (b.primary ? "primary" : "") + (b.danger ? " danger" : ""), autofocus: !!b.primary, onClick: () => { nav.dialog = null; if (b.run) R.act(b.run)(); else R.commit(); } }, b.label))))));
    }
    if (nav.toasts.length) kids.push(h("div", { class: "r-toasts", role: "status", "aria-live": "polite" }, nav.toasts.map((t) => h("div", { class: "r-toast", key: "t" + t.id }, t.text))));
    const vroot = h("div", { class: "r-app" + (ts.length > 1 ? " r-has-tabs" : "") }, kids);
    if (!root.firstChild) root.appendChild(mount(vroot)); else patch(root.firstChild, vroot);
    // appear / disappear
    const now = new Set(activeScreens().map((s) => s.name + "|" + JSON.stringify(s.args)));
    for (const s of activeScreens()) { const k = s.name + "|" + JSON.stringify(s.args); if (!lastActive.has(k) && R.screens[s.name].events.appear) R.screens[s.name].events.appear(...s.args); }
    for (const k of lastActive) if (!now.has(k)) { const [n] = k.split("|"); const sc = R.screens[n]; if (sc && sc.events.disappear) sc.events.disappear(...JSON.parse(k.slice(n.length + 1))); }
    lastActive = now;
  } finally { drawing = false; }
}
function applyTheme() {
  const t = R.theme, r = document.documentElement.style;
  const map = { accent: "--accent", background: "--bg", surface: "--surface", text: "--text", muted: "--muted", separator: "--separator", danger: "--danger", success: "--success", warning: "--warning" };
  for (const k in t) {
    if (map[k]) r.setProperty(map[k], R.show(t[k]));
    else if (k === "corner") r.setProperty("--corner", px(t[k]));
    else if (k === "spacing") r.setProperty("--space", px(t[k]));
    else if (k === "width") r.setProperty("--content-width", px(t[k]));
    else if (k === "font") r.setProperty("--font", `${JSON.stringify(R.show(t[k]))}, system-ui, sans-serif`);
    else if (k === "titlefont") r.setProperty("--title-font", `${JSON.stringify(R.show(t[k]))}, var(--font)`);
  }
}
R.start = () => {
  R.init();
  loadSaved();
  applyTheme();
  document.title = R.name;
  for (const s of R.starts) s();
  for (const t of R.timers) setInterval(t.run, t.ms);
  for (const s of Object.values(R.screens)) for (const t of s.timers) setInterval(() => { for (const a of activeScreens()) if (a.name === s.name) t.run(...a.args); }, t.ms);
  window.addEventListener("keydown", (e) => { if (e.key === "Escape" && (nav.dialog || nav.sheet)) { if (nav.dialog) nav.dialog = null; else nav.sheet = null; R.commit(); } });
  checkWatches();
  draw();
};
window.romit = { S, R, nav };
