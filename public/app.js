/* AI Hub — istemci */
const $ = (s) => document.querySelector(s);
const LS = {
  keys: 'aihub.keys', chats: 'aihub.chats', usage: 'aihub.usage',
  sel: 'aihub.selected', theme: 'aihub.theme'
};
const load = (k, d) => { try { return JSON.parse(localStorage.getItem(k)) ?? d; } catch { return d; } };
const save = (k, v) => localStorage.setItem(k, JSON.stringify(v));

let PROVIDERS = [], SERVER_KEYS = {};
let keys = load(LS.keys, {});
let chats = load(LS.chats, []);
let usage = load(LS.usage, {});
let selected = load(LS.sel, { provider: 'groq', model: 'llama-3.3-70b-versatile' });
let currentId = null;
let streaming = false;

/* ---------- kota yardımcıları ---------- */
function resetInfo(kind) {
  const now = new Date();
  if (kind === 'daily-pt') {
    // Pasifik gece yarısı ≈ UTC 08:00 (yaz saati) / 07:00 farkı önemsenmeden
    const d = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate(), 8, 0, 0));
    if (d <= now) d.setUTCDate(d.getUTCDate() + 1);
    return { at: d, label: 'Günlük (Pasifik gece yarısı)' };
  }
  if (kind === 'daily-utc') {
    const d = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate() + 1));
    return { at: d, label: 'Günlük (UTC 00:00)' };
  }
  if (kind === 'monthly') {
    const d = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth() + 1, 1));
    return { at: d, label: 'Aylık (ayın 1’i)' };
  }
  if (kind === 'credits') return { at: null, label: 'Kredi bitene kadar (yenilenmez)' };
  return { at: null, label: 'Limitsiz / bilinmiyor' };
}
function periodKey(kind) {
  const n = new Date();
  if (kind === 'monthly') return `m${n.getUTCFullYear()}-${n.getUTCMonth()}`;
  if (kind === 'credits' || kind === 'none') return 'all';
  const shift = kind === 'daily-pt' ? 8 * 3600e3 : 0;
  return 'd' + new Date(n.getTime() - shift).toISOString().slice(0, 10);
}
function countdown(date) {
  if (!date) return '—';
  let s = Math.max(0, (date - new Date()) / 1000);
  const d = Math.floor(s / 86400); s -= d * 86400;
  const h = Math.floor(s / 3600); s -= h * 3600;
  const m = Math.floor(s / 60);
  return d ? `${d}g ${h}sa` : h ? `${h}sa ${m}dk` : `${m}dk`;
}
function usageOf(pid, mid, kind) {
  const k = `${pid}|${mid}|${periodKey(kind)}`;
  return usage[k] || { req: 0, tokens: 0 };
}
function addUsage(pid, mid, kind, tokens) {
  const k = `${pid}|${mid}|${periodKey(kind)}`;
  const u = usage[k] || { req: 0, tokens: 0 };
  u.req++; u.tokens += tokens || 0;
  usage[k] = u;
  // eski dönemleri temizle
  const keep = {};
  for (const key of Object.keys(usage)) {
    const [p, m] = key.split('|');
    const mm = getModel(p, m);
    if (mm && key.endsWith(periodKey(mm.model.reset))) keep[key] = usage[key];
  }
  usage = keep; save(LS.usage, usage);
}
function getModel(pid, mid) {
  const p = PROVIDERS.find((x) => x.id === pid);
  const m = p?.models.find((x) => x.id === mid);
  return p && m ? { provider: p, model: m } : null;
}
function quotaState(pid, mid) {
  const e = getModel(pid, mid);
  if (!e) return null;
  const q = e.model.quota || {};
  const u = usageOf(pid, mid, e.model.reset);
  const limReq = q.rpd ?? q.monthly ?? null;
  const limTok = q.dailyTokens ?? q.monthlyTokens ?? q.credits ?? null;
  const pReq = limReq ? Math.min(1, u.req / limReq) : 0;
  const pTok = limTok ? Math.min(1, u.tokens / limTok) : 0;
  return { entry: e, u, limReq, limTok, pct: Math.max(pReq, pTok), reset: resetInfo(e.model.reset), rpm: q.rpm };
}
const fmt = (n) => n >= 1e9 ? (n / 1e9).toFixed(1) + 'Mr' : n >= 1e6 ? (n / 1e6).toFixed(1) + 'M' : n >= 1e3 ? (n / 1e3).toFixed(1) + 'B' : String(n);

/* ---------- anahtarlar ---------- */
const hasKey = (pid) => Boolean((keys[pid] || '').trim() || SERVER_KEYS[pid]);

/* ---------- sohbet ---------- */
function newChat() {
  currentId = null;
  renderChat();
  renderHistory();
  $('#input').focus();
}
function currentChat() { return chats.find((c) => c.id === currentId); }
function ensureChat(firstMsg) {
  if (currentChat()) return currentChat();
  const c = {
    id: 'c' + Date.now().toString(36),
    title: (firstMsg || 'Yeni sohbet').slice(0, 48),
    at: Date.now(), provider: selected.provider, model: selected.model, messages: []
  };
  chats.unshift(c); currentId = c.id;
  return c;
}
function persist() { save(LS.chats, chats.slice(0, 300)); }

function renderHistory() {
  const q = ($('#searchChats').value || '').toLowerCase();
  const list = chats.filter((c) =>
    !q || c.title.toLowerCase().includes(q) || c.messages.some((m) => m.content.toLowerCase().includes(q)));
  $('#history').innerHTML = list.map((c) => `
    <div class="hist-item ${c.id === currentId ? 'active' : ''}" data-id="${c.id}">
      <span class="t">${esc(c.title)}</span><span class="x" data-del="${c.id}">✕</span>
    </div>`).join('') || `<div style="color:var(--dim);font-size:12.5px;padding:10px">Henüz sohbet yok.</div>`;
}

function esc(s) { return String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c])); }
function md(s) {
  let h = esc(s);
  h = h.replace(/```(\w*)\n([\s\S]*?)```/g, (_, l, c) => `<pre><code>${c}</code></pre>`);
  h = h.replace(/`([^`\n]+)`/g, '<code>$1</code>');
  h = h.replace(/\*\*([^*]+)\*\*/g, '<b>$1</b>');
  return h;
}

function renderChat() {
  const c = currentChat();
  const box = $('#chat');
  if (!c || !c.messages.length) {
    box.innerHTML = '';
    box.appendChild(welcomeEl());
    return;
  }
  box.innerHTML = c.messages.map((m, i) => msgHTML(m, i)).join('');
  box.scrollTop = box.scrollHeight;
}
function msgHTML(m, i) {
  const meta = m.role === 'assistant' && m.meta ? ` · ${esc(m.meta)}` : '';
  return `<div class="msg ${m.role}">
    <div class="avatar">${m.role === 'user' ? '🙂' : '✦'}</div>
    <div class="bubble">
      <div class="who">${m.role === 'user' ? 'Sen' : esc(m.modelLabel || 'Yapay zeka')}${meta}</div>
      <div class="body ${m.error ? 'err' : ''}">${m.error ? esc(m.content) : md(m.content)}</div>
      <div class="msg-actions"><button data-copy="${i}">Kopyala</button>${m.role === 'user' ? `<button data-redo="${i}">Yeniden gönder</button>` : ''}</div>
    </div></div>`;
}
let welcomeTpl = null;
function welcomeEl() {
  if (!welcomeTpl) welcomeTpl = $('#welcome');
  const el = welcomeTpl.cloneNode(true);
  el.querySelector('#quickChips').innerHTML = [
    'Bana kısa bir iş planı yaz', 'Bu kodu açıkla', 'İngilizceye çevir', 'Beyin fırtınası yap'
  ].map((t) => `<button class="chip">${t}</button>`).join('');
  el.querySelectorAll('.chip').forEach((b) => b.onclick = () => { $('#input').value = b.textContent; $('#input').focus(); });
  return el;
}

/* ---------- model seçici ---------- */
function selLabel() {
  const e = getModel(selected.provider, selected.model);
  if (!e) return 'Model seç';
  return `${e.provider.name} · ${e.model.label}`;
}
function renderPicker() {
  const e = getModel(selected.provider, selected.model);
  $('#pickerLabel').textContent = selLabel();
  $('#pickerDot').style.background = e ? e.provider.color : 'var(--dim)';
  const q = quotaState(selected.provider, selected.model);
  if (q) {
    const left = q.limReq ? `${Math.max(0, q.limReq - q.u.req)} istek` : q.limTok ? `${fmt(Math.max(0, q.limTok - q.u.tokens))} token` : 'limit yok';
    $('#quotaPill').textContent = `${left} · ${countdown(q.reset.at)} sonra yenilenir`;
    $('#quotaPill').title = `${q.reset.label}${q.rpm ? ' · ' + q.rpm + ' istek/dk' : ''}`;
  }
  $('#composerMeta').textContent = hasKey(selected.provider)
    ? `${selLabel()} kullanılıyor` : `⚠ ${e ? e.provider.name : ''} için giriş yapman gerekiyor — Hesaplar & Anahtarlar`;
}
function renderModelList() {
  const q = ($('#modelSearch').value || '').toLowerCase();
  $('#modelList').innerHTML = PROVIDERS.map((p) => {
    const models = p.models.filter((m) =>
      !q || m.label.toLowerCase().includes(q) || m.id.toLowerCase().includes(q) || p.name.toLowerCase().includes(q));
    if (!models.length) return '';
    return `<div class="prov-group">
      <h5><span class="dot" style="background:${p.color}"></span>${esc(p.name)}
        <span class="badge ${hasKey(p.id) ? 'ok' : 'no'}">${hasKey(p.id) ? 'bağlı' : 'giriş gerekli'}</span>
        <a class="link" href="${p.keyUrl}" target="_blank" rel="noopener">anahtar al ↗</a></h5>
      <div class="mgrid">${models.map((m) => {
        const st = quotaState(p.id, m.id);
        const left = st.limReq ? `${Math.max(0, st.limReq - st.u.req)}/${st.limReq} istek`
          : st.limTok ? `${fmt(Math.max(0, st.limTok - st.u.tokens))} token kaldı` : 'limit belirsiz';
        const sel = p.id === selected.provider && m.id === selected.model;
        return `<button class="mitem ${sel ? 'sel' : ''} ${st.pct >= 1 ? 'low' : ''}" data-p="${p.id}" data-m="${m.id}">
          <b>${esc(m.label)}</b><small>${left} · ${countdown(st.reset.at)} sonra yenilenir</small></button>`;
      }).join('')}</div></div>`;
  }).join('');
}

/* ---------- kota paneli ---------- */
function renderPanel() {
  const rows = [];
  for (const p of PROVIDERS) {
    for (const m of p.models) {
      const st = quotaState(p.id, m.id);
      if (!st.u.req && !(p.id === selected.provider && m.id === selected.model)) continue;
      rows.push({ p, m, st });
    }
  }
  if (!rows.length) rows.push({ ...(() => { const e = getModel(selected.provider, selected.model); return { p: e.provider, m: e.model, st: quotaState(e.provider.id, e.model.id) }; })() });
  $('#panelBody').innerHTML = rows.map(({ p, m, st }) => {
    const pct = Math.round(st.pct * 100);
    const cls = pct >= 90 ? 'bad' : pct >= 65 ? 'warn' : '';
    const usedTxt = st.limReq ? `${st.u.req} / ${st.limReq} istek`
      : st.limTok ? `${fmt(st.u.tokens)} / ${fmt(st.limTok)} token` : `${st.u.req} istek (limit belirsiz)`;
    return `<div class="qcard">
      <h4><span class="dot" style="background:${p.color}"></span>${esc(m.label)}</h4>
      <div class="sub">${esc(p.name)} · ${st.rpm ? st.rpm + ' istek/dk' : 'hız limiti belirsiz'}</div>
      <div class="bar ${cls}"><i style="width:${pct}%"></i></div>
      <div class="qrow"><span>${usedTxt}</span><span>%${pct}</span></div>
      <div class="qrow" style="margin-top:4px"><span>${st.reset.label}</span><span>${countdown(st.reset.at)}</span></div>
    </div>`;
  }).join('');
}

/* ---------- anahtar modalı ---------- */
function renderKeys() {
  $('#keyList').innerHTML = PROVIDERS.map((p) => `
    <div class="keyrow">
      <div class="kh"><span class="dot" style="background:${p.color}"></span><b>${esc(p.name)}</b>
        <span class="badge ${hasKey(p.id) ? 'ok' : 'no'}">${SERVER_KEYS[p.id] ? 'sunucuda tanımlı' : hasKey(p.id) ? 'bağlı' : 'giriş yok'}</span></div>
      <div class="note">${esc(p.note)} <a class="link" href="${p.keyUrl}" target="_blank" rel="noopener">Ücretsiz anahtar al ↗</a></div>
      <div class="ki">
        <input type="password" placeholder="API anahtarı yapıştır…" data-key="${p.id}" value="${esc(keys[p.id] || '')}" />
        <button class="btn primary" data-save="${p.id}">Kaydet</button>
        <button class="btn" data-test="${p.id}">Test</button>
      </div>
    </div>`).join('');
}

/* ---------- gönderme ---------- */
async function send(text) {
  if (streaming) return;
  text = (text ?? $('#input').value).trim();
  if (!text) return;
  const e = getModel(selected.provider, selected.model);
  if (!e) return toast('Önce bir model seç.');
  if (!hasKey(selected.provider)) { openKeys(); return toast(`${e.provider.name} için API anahtarı gerekli.`); }

  const c = ensureChat(text);
  c.provider = selected.provider; c.model = selected.model;
  c.messages.push({ role: 'user', content: text });
  const assistant = { role: 'assistant', content: '', modelLabel: `${e.provider.name} · ${e.model.label}` };
  c.messages.push(assistant);
  $('#input').value = ''; $('#input').style.height = 'auto';
  renderChat(); renderHistory(); persist();
  streaming = true; $('#send').disabled = true;

  const bodyEl = () => $('#chat').lastElementChild?.querySelector('.body');
  bodyEl()?.classList.add('cursor');

  try {
    const res = await fetch('/api/chat', {
      method: 'POST', headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        provider: selected.provider, model: selected.model, apiKey: keys[selected.provider] || '',
        messages: c.messages.slice(0, -1).map((m) => ({ role: m.role, content: m.content })).filter((m) => m.content)
      })
    });
    const reader = res.body.getReader(); const dec = new TextDecoder(); let buf = '';
    while (true) {
      const { done, value } = await reader.read(); if (done) break;
      buf += dec.decode(value, { stream: true });
      const lines = buf.split('\n'); buf = lines.pop();
      for (const l of lines) {
        if (!l.startsWith('data:')) continue;
        const ev = JSON.parse(l.slice(5));
        if (ev.type === 'delta') {
          assistant.content += ev.text;
          const b = bodyEl(); if (b) { b.innerHTML = md(assistant.content); }
          $('#chat').scrollTop = $('#chat').scrollHeight;
        } else if (ev.type === 'error') {
          assistant.error = true; assistant.content = ev.error;
          if (ev.needKey) openKeys();
        } else if (ev.type === 'done') {
          const tok = ev.usage?.total || 0;
          assistant.meta = `${tok ? fmt(tok) + ' token · ' : ''}${(ev.ms / 1000).toFixed(1)} sn`;
          addUsage(selected.provider, selected.model, e.model.reset, tok);
        }
      }
    }
  } catch (err) {
    assistant.error = true; assistant.content = 'Bağlantı hatası: ' + err.message;
  }
  streaming = false; $('#send').disabled = false;
  persist(); renderChat(); renderPicker(); renderPanel();
}

function toast(msg) {
  const t = $('#toast'); t.textContent = msg; t.hidden = false;
  clearTimeout(t._t); t._t = setTimeout(() => (t.hidden = true), 2600);
}

/* ---------- olaylar ---------- */
function openKeys() { renderKeys(); $('#keyModal').hidden = false; }

document.addEventListener('click', async (ev) => {
  const t = ev.target;
  if (t.closest('[data-close]')) { t.closest('.modal').hidden = true; return; }
  if (t.classList.contains('modal')) { t.hidden = true; return; }

  const mi = t.closest('.mitem');
  if (mi) {
    selected = { provider: mi.dataset.p, model: mi.dataset.m };
    save(LS.sel, selected); $('#modelModal').hidden = true;
    renderPicker(); renderPanel(); return;
  }
  const del = t.dataset.del;
  if (del) { chats = chats.filter((c) => c.id !== del); if (currentId === del) currentId = null; persist(); renderHistory(); renderChat(); return; }
  const hi = t.closest('.hist-item');
  if (hi) {
    currentId = hi.dataset.id; const c = currentChat();
    if (c) { selected = { provider: c.provider, model: c.model }; save(LS.sel, selected); }
    $('#sidebar').classList.remove('open');
    renderChat(); renderHistory(); renderPicker(); return;
  }
  if (t.dataset.copy !== undefined) {
    navigator.clipboard.writeText(currentChat().messages[+t.dataset.copy].content); toast('Kopyalandı'); return;
  }
  if (t.dataset.redo !== undefined) {
    const c = currentChat(); const m = c.messages[+t.dataset.redo];
    c.messages = c.messages.slice(0, +t.dataset.redo); persist(); renderChat(); send(m.content); return;
  }
  if (t.dataset.save) {
    const v = document.querySelector(`[data-key="${t.dataset.save}"]`).value.trim();
    if (v) keys[t.dataset.save] = v; else delete keys[t.dataset.save];
    save(LS.keys, keys); renderKeys(); renderPicker(); toast('Kaydedildi'); return;
  }
  if (t.dataset.test) {
    const pid = t.dataset.test;
    const v = document.querySelector(`[data-key="${pid}"]`).value.trim();
    t.textContent = '…';
    const r = await fetch('/api/verify', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ provider: pid, apiKey: v }) }).then((r) => r.json());
    t.textContent = 'Test';
    toast(r.ok ? '✔ Anahtar çalışıyor' : '✕ ' + (r.error || 'Başarısız'));
    return;
  }
});

$('#modelPicker').onclick = () => { $('#modelSearch').value = ''; renderModelList(); $('#modelModal').hidden = false; $('#modelSearch').focus(); };
$('#modelSearch').oninput = renderModelList;
$('#openKeys').onclick = openKeys;
$('#newChat').onclick = newChat;
$('#searchChats').oninput = renderHistory;
$('#menuBtn').onclick = () => $('#sidebar').classList.toggle('open');
$('#panelBtn').onclick = () => { $('#app').classList.toggle('panel-open'); renderPanel(); };
$('#closePanel').onclick = () => $('#app').classList.remove('panel-open');
$('#send').onclick = () => send();
$('#exportBtn').onclick = () => {
  const blob = new Blob([JSON.stringify({ chats, usage }, null, 2)], { type: 'application/json' });
  const a = document.createElement('a'); a.href = URL.createObjectURL(blob);
  a.download = 'ai-hub-yedek.json'; a.click();
};
$('#input').addEventListener('input', (e) => { e.target.style.height = 'auto'; e.target.style.height = Math.min(210, e.target.scrollHeight) + 'px'; });
$('#input').addEventListener('keydown', (e) => {
  if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); send(); }
});

/* ---------- başlat ---------- */
(async function init() {
  const data = await fetch('/api/providers').then((r) => r.json());
  PROVIDERS = data.providers; SERVER_KEYS = data.serverKeys || {};
  if (!getModel(selected.provider, selected.model)) selected = { provider: PROVIDERS[0].id, model: PROVIDERS[0].models[0].id };
  welcomeTpl = $('#welcome');
  renderPicker(); renderHistory(); renderChat(); renderPanel();
  setInterval(() => { renderPicker(); if (!$('#panel').offsetParent === false) renderPanel(); }, 30000);
  if (!Object.keys(keys).length && !Object.values(SERVER_KEYS).some(Boolean)) setTimeout(openKeys, 700);
})();
