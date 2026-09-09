// AI Hub — sıfır bağımlılıklı Node.js sunucusu
// Tüm modeller bulutta çalışır; bu sunucu sadece güvenli bir proxy + statik dosya sunucusudur.

import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { PROVIDERS, findModel } from './providers.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PORT = process.env.PORT || 5173;
const HOST = process.env.HOST || '0.0.0.0';

// .env dosyasını (varsa) oku — anahtarları sunucu tarafında tutmak isteyenler için
try {
  const env = fs.readFileSync(path.join(__dirname, '.env'), 'utf8');
  for (const line of env.split('\n')) {
    const m = line.match(/^\s*([A-Z0-9_]+)\s*=\s*(.*)\s*$/);
    if (m && !process.env[m[1]]) process.env[m[1]] = m[2].replace(/^["']|["']$/g, '');
  }
} catch {}

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.json': 'application/json; charset=utf-8'
};

function serverKey(providerId) {
  return process.env[`${providerId.toUpperCase()}_API_KEY`] || '';
}

function readBody(req) {
  return new Promise((resolve, reject) => {
    let data = '';
    req.on('data', (c) => {
      data += c;
      if (data.length > 8e6) reject(new Error('İstek çok büyük'));
    });
    req.on('end', () => resolve(data));
    req.on('error', reject);
  });
}

function buildRequest({ provider, model }, apiKey, messages, opts = {}) {
  const temperature = typeof opts.temperature === 'number' ? opts.temperature : 0.7;
  if (provider.api === 'gemini') {
    const sys = messages.filter((m) => m.role === 'system').map((m) => m.content).join('\n');
    const contents = messages
      .filter((m) => m.role !== 'system')
      .map((m) => ({ role: m.role === 'assistant' ? 'model' : 'user', parts: [{ text: m.content }] }));
    return {
      url: `${provider.baseUrl}/models/${model.id}:streamGenerateContent?alt=sse&key=${encodeURIComponent(apiKey)}`,
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        contents,
        ...(sys ? { systemInstruction: { parts: [{ text: sys }] } } : {}),
        generationConfig: { temperature }
      })
    };
  }
  const headers = {
    'content-type': 'application/json',
    authorization: `Bearer ${apiKey}`
  };
  if (provider.id === 'openrouter') {
    headers['HTTP-Referer'] = 'https://github.com/LitzyDarkwin/ai-hub';
    headers['X-Title'] = 'AI Hub';
  }
  return {
    url: `${provider.baseUrl}/chat/completions`,
    headers,
    body: JSON.stringify({ model: model.id, messages, temperature, stream: true })
  };
}

function sse(res) {
  res.writeHead(200, {
    'content-type': 'text/event-stream; charset=utf-8',
    'cache-control': 'no-cache, no-transform',
    connection: 'keep-alive',
    'x-accel-buffering': 'no'
  });
  return {
    send: (obj) => res.write(`data: ${JSON.stringify(obj)}\n\n`),
    end: () => res.end()
  };
}

async function handleChat(req, res) {
  const out = sse(res);
  let payload;
  try {
    payload = JSON.parse(await readBody(req));
  } catch {
    out.send({ type: 'error', error: 'Geçersiz istek gövdesi' });
    return out.end();
  }

  const entry = findModel(payload.provider, payload.model);
  if (!entry) {
    out.send({ type: 'error', error: 'Bilinmeyen sağlayıcı/model' });
    return out.end();
  }
  const apiKey = (payload.apiKey || '').trim() || serverKey(payload.provider);
  if (!apiKey) {
    out.send({ type: 'error', error: `${entry.provider.name} için giriş yapmalısın (API anahtarı ekle).`, needKey: true });
    return out.end();
  }

  const { url, headers, body } = buildRequest(entry, apiKey, payload.messages || [], payload.options || {});
  const started = Date.now();
  let usage = null;
  let charCount = 0;

  try {
    const upstream = await fetch(url, { method: 'POST', headers, body });
    if (!upstream.ok || !upstream.body) {
      const text = await upstream.text().catch(() => '');
      let msg = text.slice(0, 600);
      try { const j = JSON.parse(text); msg = j.error?.message || j.message || msg; } catch {}
      out.send({
        type: 'error',
        status: upstream.status,
        error:
          upstream.status === 401 || upstream.status === 403
            ? `Yetki hatası: ${entry.provider.name} anahtarın geçersiz görünüyor.`
            : upstream.status === 429
            ? `Kota/hız limiti doldu (${entry.provider.name}). Başka bir model dene.`
            : `Sağlayıcı hatası ${upstream.status}: ${msg}`,
        needKey: upstream.status === 401 || upstream.status === 403,
        quotaHit: upstream.status === 429
      });
      return out.end();
    }

    const reader = upstream.body.getReader();
    const decoder = new TextDecoder();
    let buf = '';
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buf += decoder.decode(value, { stream: true });
      const parts = buf.split('\n');
      buf = parts.pop();
      for (const rawLine of parts) {
        const line = rawLine.trim();
        if (!line.startsWith('data:')) continue;
        const data = line.slice(5).trim();
        if (!data || data === '[DONE]') continue;
        let json;
        try { json = JSON.parse(data); } catch { continue; }
        let delta = '';
        if (entry.provider.api === 'gemini') {
          delta = json.candidates?.[0]?.content?.parts?.map((p) => p.text || '').join('') || '';
          if (json.usageMetadata) {
            usage = {
              input: json.usageMetadata.promptTokenCount,
              output: json.usageMetadata.candidatesTokenCount,
              total: json.usageMetadata.totalTokenCount
            };
          }
        } else {
          delta = json.choices?.[0]?.delta?.content || '';
          if (json.usage) {
            usage = { input: json.usage.prompt_tokens, output: json.usage.completion_tokens, total: json.usage.total_tokens };
          }
        }
        if (delta) {
          charCount += delta.length;
          out.send({ type: 'delta', text: delta });
        }
      }
    }
    if (!usage) usage = { total: Math.round(charCount / 4) || 1, estimated: true };
    out.send({ type: 'done', usage, ms: Date.now() - started });
  } catch (e) {
    out.send({ type: 'error', error: `Bağlantı hatası: ${e.message}` });
  }
  out.end();
}

async function handleVerify(req, res) {
  const payload = JSON.parse(await readBody(req).catch(() => '{}') || '{}');
  const provider = PROVIDERS.find((p) => p.id === payload.provider);
  const key = (payload.apiKey || '').trim() || serverKey(payload.provider || '');
  const json = (code, obj) => {
    res.writeHead(code, { 'content-type': 'application/json; charset=utf-8' });
    res.end(JSON.stringify(obj));
  };
  if (!provider) return json(400, { ok: false, error: 'Bilinmeyen sağlayıcı' });
  if (!key) return json(200, { ok: false, error: 'Anahtar boş' });
  try {
    const model = provider.models[0];
    const { url, headers, body } = buildRequest({ provider, model }, key, [{ role: 'user', content: 'ping' }]);
    const r = await fetch(url, { method: 'POST', headers, body });
    if (r.ok) { try { r.body?.cancel(); } catch {} return json(200, { ok: true }); }
    const t = await r.text().catch(() => '');
    return json(200, { ok: false, status: r.status, error: r.status === 429 ? 'Anahtar geçerli ama kota dolu' : t.slice(0, 300) });
  } catch (e) {
    return json(200, { ok: false, error: e.message });
  }
}

function serveStatic(req, res) {
  let urlPath = decodeURIComponent(new URL(req.url, 'http://x').pathname);
  if (urlPath === '/') urlPath = '/index.html';
  const filePath = path.join(__dirname, 'public', path.normalize(urlPath).replace(/^(\.\.[/\\])+/, ''));
  if (!filePath.startsWith(path.join(__dirname, 'public'))) {
    res.writeHead(403).end('Yasak');
    return;
  }
  fs.readFile(filePath, (err, data) => {
    if (err) {
      res.writeHead(404, { 'content-type': 'text/plain; charset=utf-8' }).end('Bulunamadı');
      return;
    }
    res.writeHead(200, { 'content-type': MIME[path.extname(filePath)] || 'application/octet-stream' });
    res.end(data);
  });
}

const server = http.createServer(async (req, res) => {
  res.setHeader('access-control-allow-origin', '*');
  res.setHeader('access-control-allow-headers', 'content-type');
  if (req.method === 'OPTIONS') return res.writeHead(204).end();

  const { pathname } = new URL(req.url, 'http://x');
  try {
    if (pathname === '/api/providers') {
      const configured = Object.fromEntries(PROVIDERS.map((p) => [p.id, Boolean(serverKey(p.id))]));
      res.writeHead(200, { 'content-type': 'application/json; charset=utf-8' });
      return res.end(JSON.stringify({ providers: PROVIDERS, serverKeys: configured }));
    }
    if (pathname === '/api/chat' && req.method === 'POST') return handleChat(req, res);
    if (pathname === '/api/verify' && req.method === 'POST') return handleVerify(req, res);
    return serveStatic(req, res);
  } catch (e) {
    res.writeHead(500, { 'content-type': 'text/plain; charset=utf-8' });
    res.end('Sunucu hatası: ' + e.message);
  }
});

server.listen(PORT, HOST, () => {
  console.log(`\n  AI Hub çalışıyor →  http://localhost:${PORT}\n`);
});
