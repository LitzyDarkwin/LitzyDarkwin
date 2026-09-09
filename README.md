# ✦ AI Hub

**Tüm ücretsiz ve kotalı yapay zeka modellerini tek bir sade arayüzden kullan.**

Gemini, Groq, OpenRouter, Mistral, Cerebras, GitHub Models, NVIDIA NIM, Cohere, DeepSeek, Hugging Face, SambaNova ve daha fazlası — tek pencerede, tek sohbet geçmişiyle.

> Modeller **tamamen bulutta** çalışır. Bilgisayarında hiçbir model indirilmez veya çalıştırılmaz; uygulama sadece küçük bir arayüz + güvenli proxy'dir. Zayıf bir bilgisayarda bile sorunsuz çalışır.

---

## Özellikler

- 🎛 **40+ model, 12 sağlayıcı** — tek listeden seç, anında değiştir (`Ctrl+K`, ↑↓ ile gez).
- 📊 **Kota takibi** — her model için kalan istek/token, doluluk barı ve **ne zaman yenileneceği** geri sayımı (günlük UTC, Pasifik gece yarısı, aylık, kredi bazlı). OpenRouter'da gerçek kalan kota sağlayıcıdan canlı çekilir.
- 🔁 **Otomatik yedek model** — kota dolduğunda (429) veya hata alındığında bağlı olan başka bir modelle otomatik yeniden dener.
- 💬 **Sohbet geçmişi** — otomatik kayıt, arama, tarihe göre gruplama (Bugün/Dün/Bu hafta), mesaj düzenleme, yeniden üretme, **buradan dallan**.
- 🔑 **Giriş / API anahtarı yönetimi** — tek tıkla anahtar alma linki ve "Test" butonu.
- ⚡ **Streaming yanıt** + **Durdur** butonu.
- 🎨 Markdown render: kod blokları (dil etiketi + kopyala), listeler, tablo, başlık, alıntı.
- ⚙️ **Ayarlar** — sistem talimatı, temperature, bağlam penceresi, Enter davranışı.
- 🌓 Karanlık/aydınlık tema, mobil uyumlu, yedek al/geri yükle.
- 📦 **Sıfır bağımlılık** — sadece Node.js 18+. `npm install` bile gerekmez.

### Kısayollar
`Ctrl+K` model seçici · `Ctrl+N` yeni sohbet · `Esc` pencereyi kapat · `Enter` gönder · `Shift+Enter` satır

---

## Kurulum (30 saniye)

```bash
git clone https://github.com/LitzyDarkwin/LitzyDarkwin.git ai-hub
cd ai-hub
npm start
```

Tarayıcıda aç: **http://localhost:5173**

İlk açılışta "Hesaplar & Anahtarlar" penceresi çıkar. İstediğin sağlayıcının yanındaki **"Ücretsiz anahtar al ↗"** linkine tıkla, anahtarı yapıştır, **Kaydet** → hazırsın.

### En kolay başlangıç (kredi kartı istemeyenler)

| Sağlayıcı | Anahtar linki | Ücretsiz kota |
|---|---|---|
| **Groq** | console.groq.com/keys | 14.400 istek/gün (Llama 3.1 8B) |
| **Google Gemini** | aistudio.google.com/app/apikey | 250–1000 istek/gün |
| **OpenRouter** | openrouter.ai/keys | 25+ ücretsiz model, 50 istek/gün |
| **Cerebras** | cloud.cerebras.ai | ~1M token/gün |
| **GitHub Models** | github.com/settings/tokens (`models:read`) | GPT-4o, o3, Grok — 50–150 istek/gün |
| **Mistral** | console.mistral.ai/api-keys | ~1 milyar token/ay |

---

## Anahtarların nerede duruyor?

Varsayılan olarak **yalnızca senin tarayıcının `localStorage`'ında**. Sunucuya sadece istek anında iletilir, hiçbir yere kaydedilmez ve loglanmaz.

Anahtarları sunucu tarafında tutmak istersen `.env` dosyası oluştur:

```env
GROQ_API_KEY=gsk_...
GOOGLE_API_KEY=AIza...
OPENROUTER_API_KEY=sk-or-...
MISTRAL_API_KEY=...
CEREBRAS_API_KEY=...
GITHUB_API_KEY=ghp_...
NVIDIA_API_KEY=nvapi-...
COHERE_API_KEY=...
DEEPSEEK_API_KEY=...
HUGGINGFACE_API_KEY=hf_...
SAMBANOVA_API_KEY=...
OPENAI_API_KEY=sk-...
```

Format: `<SAĞLAYICI_ID>_API_KEY`. Bu şekilde tanımlı sağlayıcılar arayüzde "sunucuda tanımlı" rozetiyle görünür.

Port değiştirmek için: `PORT=8080 npm start`

---

## Kota takibi nasıl çalışıyor?

Uygulama her isteği ve dönen token sayısını bu cihazda sayar; sağlayıcının yayınladığı resmî limitle karşılaştırıp **tahmini** kalan kotayı ve sıfırlanma zamanını gösterir. Sağlayıcılar kalan kotayı gerçek zamanlı bir API ile vermediği için bu bir tahmindir — aynı anahtarı başka bir cihazda da kullanıyorsan gerçek kullanım daha yüksek olabilir. Kesin rakam için sağlayıcının kendi panosuna bak.

Sağlayıcı 429 (kota doldu) dönerse arayüz bunu net bir mesajla söyler ve başka model seçmeni önerir.

---

## Yeni model / sağlayıcı ekleme

`providers.js` dosyasına bir kayıt ekle. OpenAI uyumlu her endpoint doğrudan çalışır:

```js
{
  id: 'yeni', name: 'Yeni Sağlayıcı', color: '#ff0000',
  keyUrl: 'https://...', note: 'Ücretsiz kota açıklaması',
  api: 'openai', baseUrl: 'https://api.ornek.com/v1',
  models: [{ id: 'model-adi', label: 'Model Adı', quota: { rpm: 30, rpd: 1000 }, reset: 'daily-utc' }]
}
```

`reset` değerleri: `daily-utc`, `daily-pt`, `monthly`, `credits`, `none`.

---

## Notlar

- Kotalar ve model listeleri sağlayıcılar tarafından sık değiştirilir; `providers.js` içindeki rakamlar Haziran 2026 itibarıyla yayınlanan bilgilere dayanır.
- Cohere'in ücretsiz deneme anahtarı ticari kullanıma kapalıdır. Google'ın ücretsiz katmanında istemler model geliştirmede kullanılabilir. Kullanmadan önce sağlayıcının şartlarını kontrol et.

## Lisans

MIT
