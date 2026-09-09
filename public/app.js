/* AI Hub — istemci v2 */
const $ = (s) => document.querySelector(s);
const $$ = (s) => [...document.querySelectorAll(s)];
const LS = {
  keys: 'aihub.keys', chats: 'aihub.chats', usage: 'aihub.usage',
  sel: 'aihub.selected', theme: 'aihub.theme', set: 'aihub.settings'
};
const load = (k, d) => { try { const v = JSON.parse(localStorage.getItem(k)); return v ?? d; } catch { return d; } };
const save = (k, v) => localStorage.setItem(k, JSON.stringify(v));

let PROVIDERS = [], SERVER_KEYS = {}, LIVE = {};
let keys = load(LS.keys, {});
let chats = load(LS.chats, []);
let usage = load(LS.usage, {});
let selected = load(LS.sel, { provider: 'groq', model: 'llama-3.3-70b-versatile' });
let settings = Object.assign(
  { sys: '', temp: 0.7, ctx: 10, autoFallback: true, enterSend: true },
  load(LS.set, {})
);
let currentId = null, streaming = false, abortCtl = null, kbIndex = -1;

/* ================= kota ================= */
function resetInfo(kind) {
  const now = new Date();
  if (kind === 'daily-pt') {
    const d = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate(), 8));
    if (d <= now) d.setUTCDate(d.getUTCDate() + 1);
    return { at: d, label: 'Günlük · Pasifik gece yarısı' };
  }
  if (kind === 'daily-utc') return { at: new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate() + 1)), label: 'Günlük · UTC 00:00' };
  if (kind === 'monthly') return { at: new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth() + 1, 1)), label: 'Aylık · ayın 1’i' };
  if (kind === 'credits') return { at: null, label: 'Kredi bazlı · yenilenmez' };
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
  let s = Math.max(0, (date - Date.now()) / 1000);
  const d = Math.floor(s / 86400); s -= d * 86400;
  const h = Math.floor(s / 3600); s -= h * 3600;
  const m = Math.floor(s / 60);
  return d ? `${d}g ${h}sa` : h ? `${h}sa ${m}dk` : `${m}dk`;
}
const getModel = (pid, mid) => {
  const p = PROVIDERS.find((x) => x.id === pid);
  const m = p?.models.find((x) => x.id === mid);
  return p && m ? { provider: p, model: m } : null;
};
const uKey = (pid, mid, kind) => `${pid}|${mid}|${periodKey(kind)}`;
function usageOf(pid, mid, kind) { return usage[uKey(pid, mid, kind)] || { req: 0, tokens: 0, min: {} }; }
function addUsage(pid, mid, kind, tokens) {
  const k = uKey(pid, mid, kind);
  const u = usage[k] || { req: 0, tokens: 0, min: {} };
  u.req++; u.tokens += tokens || 0;
  const minute = Math.floor(Date.now() / 60000);
  u.min = { [minute]: (u.min?.[minute] || 0) + 1 };
  usage[k] = u;
  for (const key of Object.keys(usage)) {
    const [p, m] = key.split('|');
    const e = getModel(p, m);
    if (!e || key !== uKey(p, m, e.model.reset)) delete usage[key];
  }
  save(LS.usage, usage);
}
function quotaState(pid, mid) {
  const e = getModel(pid, mid); if (!e) return null;
  const q = e.model.quota || {}, u = usageOf(pid, mid, e.model.reset);
  const limReq = q.rpd ?? q.monthly ?? null;
  const limTok = q.dailyTokens ?? q.monthlyTokens ?? q.credits ?? null;
  const pct = Math.max(limReq ? u.req / limReq : 0, limTok ? u.tokens / limTok : 0);
  const minute = Math.floor(Date.now() / 60000);
  return {
    entry: e, u, limReq, limTok, pct: Math.min(1, pct), reset: resetInfo(e.model.reset),
    rpm: q.rpm, thisMinute: u.min?.[minute] || 0, exhausted: pct >= 1
  };
}
const fmt = (n) => n >= 1e9 ? (n / 1e9).toFixed(1) + 'Mr' : n >= 1e6 ? (n / 1e6).toFixed(1) + 'M' : n >= 1e3 ? (n / 1e3).toFixed(1) + 'B' : String(Math.round(n));
const hasKey = (pid) => Boolean((keys[pid] || '').trim() || SERVER_KEYS[pid]);

/* ================= markdown ================= */
const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
function md(src) {
  const blocks = [];
  let s = String(src).replace(/```(\w*)\n?([\s\S]*?)(?:```|$)/g, (_, lang, code) => {
    blocks.push(`<div class="codewrap"><span class="lang">${esc(lang || 'kod')}</span><button class="cbtn" data-code="${blocks.length}">kopyala</button><pre><code>${esc(code.replace(/\n$/, ''))}</code></pre></div>`);
    return `\u0000B${blocks.length - 1}\u0000`;
  });
  s = esc(s);
  s = s.replace(/^### (.+)$/gm, '<h3>$1</h3>')
       .replace(/^## (.+)$/gm, '<h3>$1</h3>')
       .replace(/^&gt; (.+)$/gm, '<blockquote>$1</blockquote>')
       .replace(/`([^`\n]+)`/g, '<code>$1</code>')
       .replace(/\*\*([^*]+)\*\*/g, '<b>$1</b>')
       .replace(/(^|[\s(])\*([^*\n]+)\*/g, '$1<i>$2</i>')
       .replace(/\[([^\]]+)\]\((https?:\/\/[^)\s]+)\)/g, '<a class="link" target="_blank" rel="noopener" href="$2">$1</a>');
  s = s.replace(/(?:^[-*] .+\n?)+/gm, (m2) => '<ul>' + m2.trim().split('\n').map((l) => `<li>${l.replace(/^[-*] /, '')}</li>`).join('') + '</ul>');
  s = s.replace(/(?:^\d+\. .+\n?)+/gm, (m2) => '<ol>' + m2.trim().split('\n').map((l) => `<li>${l.replace(/^\d+\. /, '')}</li>`).join('') + '</ol>');
  s = s.replace(/\n?(<\/(?:ul|ol|h3|blockquote)>)\n?/g, '$1').replace(/\n?(<(?:ul|ol|h3|blockquote)>)/g, '$1');
  return s.replace(/\u0000B(\d+)\u0000/g, (_, i) => blocks[+i]);
}

/* ================= sohbet ================= */
const currentChat = () => chats.find((c) => c.id === currentId);
function newChat() { currentId = null; renderChat(); renderHistory(); $('#input').focus(); }
function ensureChat(first) {
  const c0 = currentChat(); if (c0) return c0;
  const c = { id: 'c' + Date.now().toString(36), title: (first || 'Yeni sohbet').replace(/\s+/g, ' ').slice(0, 48), at: Date.now(), provider: selected.provider, model: selected.model, messages: [] };
  chats.unshift(c); currentId = c.id; return c;
}
const persist = () => save(LS.chats, chats.slice(0, 300));

function groupLabel(ts) {
  const d = new Date(ts), n = new Date();
  const day = (a) => new Date(a.getFullYear(), a.getMonth(), a.getDate()).getTime();
  const diff = (day(n) - day(d)) / 86400000;
  return diff <= 0 ? 'Bugün' : diff === 1 ? 'Dün' : diff < 7 ? 'Bu hafta' : diff < 30 ? 'Bu ay' : 'Daha eski';
}
function renderHistory() {
  const q = ($('#searchChats').value || '').toLowerCase();
  const list = chats.filter((c) => !q || c.title.toLowerCase().includes(q) || c.messages.some((m) => (m.content || '').toLowerCase().includes(q)));
  let html = '', last = '';
  for (const c of list) {
    const g = groupLabel(c.at || Date.now());
    if (g !== last) { html += `<div class="hist-group">${g}</div>`; last = g; }
    const e = getModel(c.provider, c.model);
    html += `<div class="hist-item ${c.id === currentId ? 'active' : ''}" data-id="${c.id}">
      <span class="t">${esc(c.title)}<br><span class="m">${e ? esc(e.model.label) : ''} · ${c.messages.length} mesaj</span></span>
      <span class="x" data-del="${c.id}">✕</span></div>`;
  }
  $('#history').innerHTML = html || `<div style="color:var(--dim);font-size:12.5px;padding:10px">${q ? 'Sonuç yok.' : 'Henüz sohbet yok.'}</div>`;
}

let welcomeTpl = null;
function welcomeEl() {
  const el = welcomeTpl.cloneNode(true); el.removeAttribute('hidden');
  el.querySelector('#quickChips').innerHTML = ['Bana kısa bir iş planı yaz', 'Bu kodu açıkla', 'İngilizceye çevir', 'Beyin fırtınası yap', 'Özetle']
    .map((t) => `<button class="chip">${esc(t)}</button>`).join('');
  el.querySelectorAll('.chip').forEach((b) => b.onclick = () => { $('#input').value = b.textContent; $('#input').dispatchEvent(new Event('input')); $('#input').focus(); });
  return el;
}
function msgHTML(m, i) {
  const meta = m.role === 'assistant' && m.meta ? ` · ${esc(m.meta)}` : '';
  const acts = m.role === 'user'
    ? `<button data-copy="${i}">Kopyala</button><button data-edit="${i}">Düzenle</button><button data-redo="${i}">Yeniden gönder</button>`
    : `<button data-copy="${i}">Kopyala</button><button data-regen="${i}">Yeniden üret</button><button data-branch="${i}">Buradan dallan</button>`;
  return `<div class="msg ${m.role}" data-i="${i}">
    <div class="avatar">${m.role === 'user' ? '🙂' : '✦'}</div>
    <div class="bubble">
      <div class="who">${m.role === 'user' ? 'Sen' : esc(m.modelLabel || 'Yapay zeka')}${meta}</div>
      <div class="body ${m.error ? 'err' : ''}">${m.error ? esc(m.content) : md(m.content)}</div>
      <div class="msg-actions">${acts}</div>
    </div></div>`;
}
function renderChat() {
  const c = currentChat(), box = $('#chat');
  $('#regenBtn').hidden = !(c && c.messages.length >= 2 && !streaming);
  if (!c || !c.messages.length) { box.innerHTML = ''; box.appendChild(welcomeEl()); return; }
  box.innerHTML = c.messages.map(msgHTML).join('');
  box.scrollTop = box.scrollHeight;
}

/* ================= üst bar ================= */
function selLabel() { const e = getModel(selected.provider, selected.model); return e ? `${e.provider.name} · ${e.model.label}` : 'Model seç'; }
function renderPicker() {
  const e = getModel(selected.provider, selected.model);
  $('#pickerLabel').textContent = selLabel();
  $('#pickerDot').style.background = e ? e.provider.color : 'var(--dim)';
  const q = quotaState(selected.provider, selected.model);
  if (q) {
    const live = LIVE[selected.provider];
    const left = live?.remaining != null ? `${fmt(live.remaining)} kredi`
      : q.limReq ? `${Math.max(0, q.limReq - q.u.req)} istek`
      : q.limTok ? `${fmt(Math.max(0, q.limTok - q.u.tokens))} token` : 'limit yok';
    $('#quotaPill').textContent = `${left} · ${countdown(q.reset.at)}`;
    $('#quotaPill').title = `${q.reset.label}${q.rpm ? ' · ' + q.rpm + ' istek/dk' : ''}`;
    $('#quotaPill').style.color = q.pct >= .9 ? 'var(--bad)' : q.pct >= .65 ? 'var(--warn)' : '';
  }
  $('#composerMeta').innerHTML = hasKey(selected.provider)
    ? `${esc(selLabel())} kullanılıyor`
    : `<span style="color:var(--warn)">⚠ ${e ? esc(e.provider.name) : ''} için giriş gerekli</span>`;
}
function renderModelList() {
  const q = ($('#modelSearch').value || '').toLowerCase(), only = $('#onlyReady').checked;
  let idx = 0;
  $('#modelList').innerHTML = PROVIDERS.map((p) => {
    if (only && !hasKey(p.id)) return '';
    const models = p.models.filter((m) => !q || m.label.toLowerCase().includes(q) || m.id.toLowerCase().includes(q) || p.name.toLowerCase().includes(q));
    if (!models.length) return '';
    return `<div class="prov-group">
      <h5><span class="dot" style="background:${p.color}"></span>${esc(p.name)}
        <span class="badge ${hasKey(p.id) ? 'ok' : 'no'}">${hasKey(p.id) ? 'bağlı' : 'giriş gerekli'}</span>
        <a class="link" href="${p.keyUrl}" target="_blank" rel="noopener">anahtar al ↗</a></h5>
      <div class="mgrid">${models.map((m) => {
        const st = quotaState(p.id, m.id), pct = Math.round(st.pct * 100);
        const left = st.limReq ? `${Math.max(0, st.limReq - st.u.req)}/${st.limReq} istek`
          : st.limTok ? `${fmt(Math.max(0, st.limTok - st.u.tokens))} token kaldı` : 'limit belirsiz';
        const sel = p.id === selected.provider && m.id === selected.model;
        return `<button class="mitem ${sel ? 'sel' : ''} ${st.exhausted ? 'low' : ''}" data-idx="${idx++}" data-p="${p.id}" data-m="${m.id}">
          <b>${esc(m.label)}</b><small>${left} · ${countdown(st.reset.at)} sonra yenilenir</small>
          <div class="qbar ${pct >= 90 ? 'bad' : pct >= 65 ? 'warn' : ''}"><i style="width:${pct}%"></i></div>
          <div class="tags">${st.rpm ? `<span class="tag">${st.rpm}/dk</span>` : ''}${st.exhausted ? '<span class="tag">kota doldu</span>' : ''}${!hasKey(p.id) ? '<span class="tag">🔒</span>' : ''}</div>
        </button>`;
      }).join('')}</div></div>`;
  }).join('') || `<div style="color:var(--dim);padding:20px;text-align:center">Eşleşen model yok.</div>`;
  kbIndex = -1;
}
function renderPanel() {
  if (!$('#app').classList.contains('panel-open')) return;
  const rows = [];
  for (const p of PROVIDERS) for (const m of p.models) {
    const st = quotaState(p.id, m.id);
    if (st.u.req || (p.id === selected.provider && m.id === selected.model)) rows.push({ p, m, st });
  }
  rows.sort((a, b) => b.st.u.req - a.st.u.req);
  const totalReq = rows.reduce((s, r) => s + r.st.u.req, 0);
  const totalTok = rows.reduce((s, r) => s + r.st.u.tokens, 0);
  $('#panelBody').innerHTML =
    `<div class="qcard"><h4>Bu dönemki toplam</h4><div class="qrow"><span>${totalReq} istek</span><span>${fmt(totalTok)} token</span></div></div>` +
    rows.map(({ p, m, st }) => {
      const pct = Math.round(st.pct * 100);
      const cls = pct >= 90 ? 'bad' : pct >= 65 ? 'warn' : '';
      const used = st.limReq ? `${st.u.req} / ${st.limReq} istek` : st.limTok ? `${fmt(st.u.tokens)} / ${fmt(st.limTok)} token` : `${st.u.req} istek`;
      const live = LIVE[p.id]?.remaining != null ? `<div class="qrow" style="margin-top:4px;color:var(--ok)"><span>Sağlayıcıdan canlı</span><span>${fmt(LIVE[p.id].remaining)} kalan</span></div>` : '';
      return `<div class="qcard">
        <h4><span class="dot" style="background:${p.color}"></span>${esc(m.label)}</h4>
        <div class="sub">${esc(p.name)} · ${st.rpm ? st.rpm + ' istek/dk' : 'hız limiti belirsiz'}${st.thisMinute ? ` · bu dk ${st.thisMinute}` : ''}</div>
        <div class="bar ${cls}"><i style="width:${pct}%"></i></div>
        <div class="qrow"><span>${used}</span><span>%${pct}</span></div>
        <div class="qrow" style="margin-top:4px"><span>${st.reset.label}</span><span>${countdown(st.reset.at)}</span></div>${live}
      </div>`;
    }).join('');
}
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

/* ================= gönderme ================= */
function pickFallback(excl) {
  const cands = [];
  for (const p of PROVIDERS) {
    if (!hasKey(p.id)) continue;
    for (const m of p.models) {
      if (excl.has(p.id + '|' + m.id)) continue;
      const st = quotaState(p.id, m.id);
      if (st.exhausted) continue;
      cands.push({ p, m, free: st.limReq ? st.limReq - st.u.req : 1e9 });
    }
  }
  cands.sort((a, b) => b.free - a.free);
  return cands[0] || null;
}

async function streamOnce(pid, mid, msgs, assistant, onDelta) {
  const e = getModel(pid, mid);
  abortCtl = new AbortController();
  const res = await fetch('/api/chat', {
    method: 'POST', headers: { 'content-type': 'application/json' }, signal: abortCtl.signal,
    body: JSON.stringify({
      provider: pid, model: mid, apiKey: keys[pid] || '',
      options: { temperature: settings.temp },
      messages: (settings.sys ? [{ role: 'system', content: settings.sys }] : []).concat(msgs)
    })
  });
  const reader = res.body.getReader(), dec = new TextDecoder();
  let buf = '', err = null, doneEv = null;
  while (true) {
    const { done, value } = await reader.read(); if (done) break;
    buf += dec.decode(value, { stream: true });
    const lines = buf.split('\n'); buf = lines.pop();
    for (const l of lines) {
      if (!l.startsWith('data:')) continue;
      let ev; try { ev = JSON.parse(l.slice(5)); } catch { continue; }
      if (ev.type === 'delta') { assistant.content += ev.text; onDelta(); }
      else if (ev.type === 'error') err = ev;
      else if (ev.type === 'done') doneEv = ev;
    }
  }
  if (doneEv) addUsage(pid, mid, e.model.reset, doneEv.usage?.total || 0);
  return { err, doneEv };
}

async function send(text) {
  if (streaming) return;
  text = (text ?? $('#input').value).trim(); if (!text) return;
  const e = getModel(selected.provider, selected.model);
  if (!e) return toast('Önce bir model seç.');
  if (!hasKey(selected.provider)) { openKeys(); return toast(`${e.provider.name} için API anahtarı gerekli.`); }
  const c = ensureChat(text);
  c.provider = selected.provider; c.model = selected.model; c.at = Date.now();
  c.messages.push({ role: 'user', content: text });
  $('#input').value = ''; $('#input').style.height = 'auto';
  await run(c);
}

async function run(c) {
  const e = getModel(selected.provider, selected.model);
  const assistant = { role: 'assistant', content: '', modelLabel: `${e.provider.name} · ${e.model.label}` };
  c.messages.push(assistant);
  renderChat(); renderHistory(); persist();
  streaming = true; setBusy(true);

  const bodyEl = () => $('#chat').lastElementChild?.querySelector('.body');
  const b0 = bodyEl(); if (b0) b0.innerHTML = '<span class="typing"><i></i><i></i><i></i></span>';
  const onDelta = () => { const b = bodyEl(); if (b) b.innerHTML = md(assistant.content) + '<span class="cursor"></span>'; $('#chat').scrollTop = $('#chat').scrollHeight; };

  const history = c.messages.slice(0, -1).filter((m) => m.content && !m.error).slice(-settings.ctx).map((m) => ({ role: m.role, content: m.content }));
  const tried = new Set();
  let pid = selected.provider, mid = selected.model, r = null;

  for (let attempt = 0; attempt < 3; attempt++) {
    tried.add(pid + '|' + mid);
    try { r = await streamOnce(pid, mid, history, assistant, onDelta); }
    catch (err) {
      if (err.name === 'AbortError') { assistant.meta = 'durduruldu'; break; }
      r = { err: { error: 'Bağlantı hatası: ' + err.message } };
    }
    if (!r.err) break;
    if (r.err.needKey) { openKeys(); break; }
    if (!settings.autoFallback) break;
    const fb = pickFallback(tried);
    if (!fb) break;
    pid = fb.p.id; mid = fb.m.id;
    assistant.modelLabel = `${fb.p.name} · ${fb.m.label}`;
    assistant.content = '';
    toast(`Kota/hata → ${fb.m.label} ile deneniyor…`);
    renderChat();
  }

  if (r?.err) { assistant.error = true; assistant.content = r.err.error; }
  else if (r?.doneEv) {
    const t = r.doneEv.usage?.total || 0;
    assistant.meta = `${t ? fmt(t) + ' token · ' : ''}${(r.doneEv.ms / 1000).toFixed(1)} sn${r.doneEv.aborted ? ' · durduruldu' : ''}`;
  }
  streaming = false; abortCtl = null; setBusy(false);
  persist(); renderChat(); renderPicker(); renderPanel(); refreshLive();
}

function setBusy(b) { $('#send').hidden = b; $('#stop').hidden = !b; $('#regenBtn').hidden = b; }
function toast(msg) { const t = $('#toast'); t.textContent = msg; t.hidden = false; clearTimeout(t._t); t._t = setTimeout(() => (t.hidden = true), 2800); }
function openKeys() { renderKeys(); $('#keyModal').hidden = false; }
function openModels() { $('#modelSearch').value = ''; renderModelList(); $('#modelModal').hidden = false; $('#modelSearch').focus(); }

async function refreshLive() {
  for (const pid of ['openrouter', 'huggingface']) {
    if (!hasKey(pid)) continue;
    try {
      const r = await fetch('/api/quota', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ provider: pid, apiKey: keys[pid] || '' }) }).then((x) => x.json());
      if (r.ok) LIVE[pid] = r;
    } catch {}
  }
  renderPicker(); renderPanel();
}

/* ================= olaylar ================= */
document.addEventListener('click', async (ev) => {
  const t = ev.target;
  if (t.closest('[data-close]')) { t.closest('.modal').hidden = true; return; }
  if (t.classList.contains('modal')) { t.hidden = true; return; }
  if (t.dataset.code !== undefined) {
    const code = t.parentElement.querySelector('code').textContent;
    navigator.clipboard.writeText(code); t.textContent = 'kopyalandı ✓';
    setTimeout(() => (t.textContent = 'kopyala'), 1500); return;
  }
  const mi = t.closest('.mitem');
  if (mi) { selected = { provider: mi.dataset.p, model: mi.dataset.m }; save(LS.sel, selected); $('#modelModal').hidden = true; renderPicker(); renderPanel(); return; }
  if (t.dataset.del) {
    const id = t.dataset.del; chats = chats.filter((c) => c.id !== id);
    if (currentId === id) currentId = null;
    persist(); renderHistory(); renderChat(); return;
  }
  const hi = t.closest('.hist-item');
  if (hi) {
    currentId = hi.dataset.id; const c = currentChat();
    if (c && getModel(c.provider, c.model)) { selected = { provider: c.provider, model: c.model }; save(LS.sel, selected); }
    $('#sidebar').classList.remove('open');
    renderChat(); renderHistory(); renderPicker(); return;
  }
  const c = currentChat();
  if (t.dataset.copy !== undefined && c) { navigator.clipboard.writeText(c.messages[+t.dataset.copy].content); toast('Kopyalandı'); return; }
  if (t.dataset.edit !== undefined && c) {
    const i = +t.dataset.edit; $('#input').value = c.messages[i].content;
    c.messages = c.messages.slice(0, i); persist(); renderChat();
    $('#input').dispatchEvent(new Event('input')); $('#input').focus(); return;
  }
  if (t.dataset.redo !== undefined && c) { const i = +t.dataset.redo, m = c.messages[i]; c.messages = c.messages.slice(0, i); persist(); renderChat(); send(m.content); return; }
  if (t.dataset.regen !== undefined && c) { c.messages = c.messages.slice(0, +t.dataset.regen); persist(); run(c); return; }
  if (t.dataset.branch !== undefined && c) {
    const copy = { ...c, id: 'c' + Date.now().toString(36), at: Date.now(), title: '↳ ' + c.title, messages: c.messages.slice(0, +t.dataset.branch + 1).map((x) => ({ ...x })) };
    chats.unshift(copy); currentId = copy.id; persist(); renderChat(); renderHistory(); toast('Yeni dal oluşturuldu'); return;
  }
  if (t.dataset.save) {
    const v = document.querySelector(`[data-key="${t.dataset.save}"]`).value.trim();
    if (v) keys[t.dataset.save] = v; else delete keys[t.dataset.save];
    save(LS.keys, keys); renderKeys(); renderPicker(); refreshLive(); toast('Kaydedildi'); return;
  }
  if (t.dataset.test) {
    const pid = t.dataset.test, v = document.querySelector(`[data-key="${pid}"]`).value.trim();
    t.textContent = '…';
    const r = await fetch('/api/verify', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ provider: pid, apiKey: v }) }).then((x) => x.json()).catch(() => ({ ok: false, error: 'ağ hatası' }));
    t.textContent = 'Test'; toast(r.ok ? '✔ Anahtar çalışıyor' : '✕ ' + (r.error || 'Başarısız')); return;
  }
});

$('#modelPicker').onclick = openModels;
$('#modelSearch').oninput = renderModelList;
$('#onlyReady').onchange = renderModelList;
$('#modelSearch').onkeydown = (e) => {
  const items = $$('.mitem');
  if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
    e.preventDefault();
    kbIndex = Math.max(0, Math.min(items.length - 1, kbIndex + (e.key === 'ArrowDown' ? 1 : -1)));
    items.forEach((el, i) => el.classList.toggle('kb', i === kbIndex));
    items[kbIndex]?.scrollIntoView({ block: 'nearest' });
  } else if (e.key === 'Enter') { e.preventDefault(); (items[kbIndex] || items[0])?.click(); }
  else if (e.key === 'Escape') $('#modelModal').hidden = true;
};
$('#openKeys').onclick = openKeys;
$('#openSettings').onclick = () => {
  $('#sysPrompt').value = settings.sys; $('#temp').value = settings.temp; $('#tempVal').textContent = settings.temp;
  $('#ctx').value = settings.ctx; $('#ctxVal').textContent = settings.ctx;
  $('#autoFallback').checked = settings.autoFallback; $('#enterSend').checked = settings.enterSend;
  $('#setModal').hidden = false;
};
const setSet = (k, v) => { settings[k] = v; save(LS.set, settings); };
$('#sysPrompt').oninput = (e) => setSet('sys', e.target.value);
$('#temp').oninput = (e) => { setSet('temp', +e.target.value); $('#tempVal').textContent = e.target.value; };
$('#ctx').oninput = (e) => { setSet('ctx', +e.target.value); $('#ctxVal').textContent = e.target.value; };
$('#autoFallback').onchange = (e) => setSet('autoFallback', e.target.checked);
$('#enterSend').onchange = (e) => setSet('enterSend', e.target.checked);
$('#newChat').onclick = newChat;
$('#searchChats').oninput = renderHistory;
$('#menuBtn').onclick = () => $('#sidebar').classList.toggle('open');
$('#panelBtn').onclick = () => { $('#app').classList.toggle('panel-open'); renderPanel(); };
$('#closePanel').onclick = () => $('#app').classList.remove('panel-open');
$('#resetUsage').onclick = () => { if (confirm('Kota sayaçları sıfırlansın mı?')) { usage = {}; save(LS.usage, usage); renderPanel(); renderPicker(); } };
$('#send').onclick = () => send();
$('#stop').onclick = () => { abortCtl?.abort(); toast('Durduruldu'); };
$('#regenBtn').onclick = () => { const c = currentChat(); if (!c) return; while (c.messages.length && c.messages.at(-1).role === 'assistant') c.messages.pop(); run(c); };
$('#themeBtn').onclick = () => {
  const cur = document.documentElement.dataset.theme === 'light' ? 'dark' : 'light';
  document.documentElement.dataset.theme = cur; save(LS.theme, cur);
};
$('#exportBtn').onclick = () => {
  const blob = new Blob([JSON.stringify({ v: 2, chats, usage, settings }, null, 2)], { type: 'application/json' });
  const a = document.createElement('a'); a.href = URL.createObjectURL(blob);
  a.download = `ai-hub-yedek-${new Date().toISOString().slice(0, 10)}.json`; a.click();
};
$('#importBtn').onclick = () => $('#importFile').click();
$('#importFile').onchange = async (e) => {
  const f = e.target.files[0]; if (!f) return;
  try {
    const d = JSON.parse(await f.text());
    if (Array.isArray(d.chats)) { chats = d.chats.concat(chats); persist(); }
    if (d.usage) { usage = Object.assign(usage, d.usage); save(LS.usage, usage); }
    if (d.settings) { settings = Object.assign(settings, d.settings); save(LS.set, settings); }
    renderHistory(); renderPanel(); toast('Yedek yüklendi');
  } catch { toast('Dosya okunamadı'); }
  e.target.value = '';
};
$('#input').addEventListener('input', (e) => { e.target.style.height = 'auto'; e.target.style.height = Math.min(210, e.target.scrollHeight) + 'px'; updHint(); });
$('#input').addEventListener('keydown', (e) => {
  const go = settings.enterSend ? (e.key === 'Enter' && !e.shiftKey) : (e.key === 'Enter' && (e.ctrlKey || e.metaKey));
  if (go) { e.preventDefault(); send(); }
});
function updHint() {
  const n = $('#input').value.length;
  $('#tokenHint').textContent = n ? `~${Math.ceil(n / 4)} token` : '';
}
document.addEventListener('keydown', (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); openModels(); }
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'n') { e.preventDefault(); newChat(); }
  if (e.key === 'Escape') $$('.modal').forEach((m) => (m.hidden = true));
});

/* ================= başlat ================= */
(async function init() {
  document.documentElement.dataset.theme = load(LS.theme, 'dark');
  welcomeTpl = $('#welcome');
  const data = await fetch('/api/providers').then((r) => r.json());
  PROVIDERS = data.providers; SERVER_KEYS = data.serverKeys || {};
  if (!getModel(selected.provider, selected.model)) selected = { provider: PROVIDERS[0].id, model: PROVIDERS[0].models[0].id };
  renderPicker(); renderHistory(); renderChat();
  setInterval(() => { renderPicker(); renderPanel(); }, 30000);
  refreshLive();
  if (!Object.keys(keys).length && !Object.values(SERVER_KEYS).some(Boolean)) setTimeout(openKeys, 700);
})();
