# Hesabı Var — 10 Ajanlı Çok Perspektifli Analiz ve Sentez

> Kaynak: kullanıcı tarafından sağlanan proje brifingi (CLAUDE.md/DURUM.md mimarisi, 2026-10-16 durumu). Depo şu an yalnızca `README.md` içerdiği için analiz, brifingde tanımlanan sistem, mimari ve iş akışı üzerinden yapılmıştır. Kod tabanı erişilemediğinde "kod seviyesi" bulgular mimari çıkarım olarak işaretlenmiştir.

---

## 0. Yönetici Özeti (tek paragraf)

"Hesabı Var", AI-üretimli, kaynaklı veriye ve kodla çizilen hareketli grafiklere dayalı Türkçe bir ekonomi YouTube kanalıdır. Mimari olarak **araştırma → senaryo → ses (TTS+Whisper) → Remotion render → kalite filtresi → teslim** şeklinde fazlı, tek-faz-per-oturum bir pipeline'a sahiptir. Güçlü yönleri: kaynak disiplini, sese kilitli görüntü mimarisi, 3 tur üst üste yeşil kalite kapısı, bellek tabanlı kendi-kendini iyileştirme. Temel riskler: tek kişi (kullanıcı) onay/bağımlılık noktası, ElevenLabs geçişinin maliyet ve telaffuz riski, kalite filtresinin subjektif "yeşil" tanımı, ve henüz ölçülebilir performans verisinin (ANALYTICS_LOG) olgunlaşmamış olması. Sentez sonucu: **proje teknik olarak olgun ve tutarlı; büyüme ve sürdürülebilirlik için en kritik eksikler ölçüm (analytics) ve otomasyon (onay döngüsü) katmanlarıdır.**

---

## 1. Ajan 1 — Ürün & Strateji Analisti

**Kapsam:** Konumlandırma, vaat, içerik sütunları, hedef kitle, rekabet.

**Bulgular:**
- Vaat ("Ödediğin her fiyatın bir hesabı var") net, ölçülebilir ve niş: gündelik ekonomi + kaynaklı veri + kodla çizim. Bu üçlü, stok B-roll ve AI görsel kullanan rakiplerden ayırt edici.
- 3 sütun (SH/FA/KA) süre ve şablon farkıyla çeşitlilik sağlıyor; ancak hepsi "hesap açma" aynı çekirdek vaadi paylaşıyor — kanal kimliği güçlü.
- Tempo "haftada en çok 2 uzun video" + kalite filtresi geçmeyen yayınlanmaz → kalite > hacim stratejisi; erken kanal için doğru ama büyüme hızını sınırlar.
- "Teslimden sonra AI durur; sonraki video kullanıcı onayıyla başlar" → kullanıcı tek darboğaz; ölçeklenebilirlik riski.
- Konsept kilitli, değişiklik ancak analitik veriyle ve 10. videodan sonra → disiplinli, ama erken öğrenme döngüsünü geciktirebilir.

**Skor:** 8/10. **Öneriler:** (1) 10. video öncesi hafif A/B (başlık/kapak) testleriyle erken öğrenme; (2) kullanıcı onayını asenkron checklist'e bağlayıp AI'nın paralel araştırma yapmasını izin ver; (3) Shorts'u büyüme motoru olarak konumlandır (uzun videodan türetim zaten var).

---

## 2. Ajan 2 — Teknik Mimari Analisti

**Kapsam:** Remotion motoru, Python hattı, veri akışı, bağımlılıklar.

**Bulgular:**
- Mimari fikir güçlü: görüntü sese bağlı (`zamanlama.json` + `ara()` kelime anı). "Rakam söylendiği anda ekranda belirir" ve "ekran cevabı erken vermez" → anlatı-görüntü senkronu teknik olarak garanti altına alınmış.
- Tek bundle'dan çok kare çıkarma (`kareler.mjs`) → önizleme maliyetini düşürür.
- `theme.ts` ↔ `ART_DIRECTION.md` eşleşmesi → tasarım token'larının kodla senkronu; drift riski az.
- Risk: Remotion 4 + React 18 + TS; `tsc` kontrolü render öncesi → iyi. Ancak render süresi/ maliyeti (lokal GPU) ölçülmemiş.
- Risk: `public/VNNN/zamanlama.json` kopyalama adımı manuel/ betik bağımlı; tek bir bozuk kopya tüm render'ı etkiler.
- V004 ElevenLabs geçişi: `ses.sh` TTS kısmı yeniden yazılacak → geçiş riski; Chatterbox referans dosyası değiştirilmez kuralı iyi.

**Skor:** 7,5/10. **Öneriler:** (1) `zamanlama.json` şeması için JSON schema + doğrulama adımı; (2) render süresi/maliyeti loglama; (3) ElevenLabs adaptörü için soyut TTS arayüzü (Chatterbox ile değiştirilebilir).

---

## 3. Ajan 3 — İçerik & Editöryal Kalite Analisti

**Kapsam:** Senaryo kuralları, yapaylık filtreleri F1–F13, klişe/söylem işaretleyicisi limitleri.

**Bulgular:**
- Cümle ≤18 kelime, rakamlar kelimeyle, kaynak satırı tek satır, söylem işaretleyicisi ≤2 kez → ölçülebilir editöryal standartlar; kalite_kontrol.py ile otomatikleştirilmiş.
- Yapaylık filtresi 5 tur (A–E) ve 3 tur üst üste yeşil kapısı → V003'te 6 tur gerekmiş → filtre gerçekten işliyor ama maliyetli.
- Risk: "yeşil" tanımı kısmen subjektif (özellikle A doğallık ve B izleyici akışı); otomatik kontroller (klişe sayımı) objektif ama "kanca etkisi" gibi maddeler insan/AI yargısına dayalı.
- Risk: AUDIENCE_VOICE.md yasaklı klişe listesi zamanla eskir; güncel trend klişeleri (ör. "Bakın", "Şunu bilin") eklenmeli.

**Skor:** 8/10. **Öneriler:** (1) Kalite filtresi maddelerini puanlı rubrik (0–3) ile sayısal hale getir; (2) klişe listesini her videoda güncel trend taramasıyla genişlet; (3) "3 tur yeşil" kuralını "2 tur yeşil + 1 tur bağımsız gözden geçirme" olarak gevşetmeyi değerlendir (maliyet).

---

## 4. Ajan 4 — Kanal Büyüme & Pazar Analisti

**Kapsam:** MARKET_INTEL, FIKIR_HAVUZU, performans, Shorts.

**Bulgular:**
- 100+ konu fikri havuzu + pazar/ rakip istihbaratı → içerik planlama disiplinli.
- V001–V003 farklı sütunlardan → erken çeşitlilik; hangi sütunun daha iyi performans gösterdiği henüz ölçülmemiş (ANALYTICS_LOG olgunlaşmamış).
- Shorts 25–60 sn dikey, içerik %10–%58 bandı, sağ buton sütunu boş → platform uyumlu.
- Risk: Sabit yayın günü yok → algoritma için tutarsızlık; "haftada en çok 2" esnekliği iyi ama minimum tutarlılık (ör. haftada 1) tanımlanmamış.
- Risk: Konu seçimi AI'da; kullanıcı onayıyla başlıyor → kullanıcı tercihleri kanal stratejisini etkileyebilir (pozitif de olabilir).

**Skor:** 7/10. **Öneriler:** (1) Minimum haftalık 1 video taahhüdü; (2) her videodan sonra 7/30/90 gün metrik toplama şablonu; (3) Shorts → uzun video hunisi (açıklama linki, pinned comment) standartlaştırma.

---

## 5. Ajan 5 — İzleyici Deneyimi (UX) Analisti

**Kapsam:** Görsel kimlik, güvenli alan, altyazı, ses.

**Bulgular:**
- Koyu zemin + kareli hesap kâğıdı + amber vurgu + termal kasa fişi imzası → güçlü, tutarlı görsel kimlik; "Hesap Fişi" kapanış sahnesi marka hafızası oluşturur.
- Güvenli alan %5, dış %1,2 şerit boş (KENAR_MASKE) → kenar taşması riski azaltılmış; kalite kontrolde de ölçülmüş.
- Altyazı senkronu ve `bol()` sayfa bölme (cümle sonu + rakam-birim önceliği) → okunabilirlik ve senkron iyi.
- Risk: Shorts'ta içerik %10–%58 bandı dar; uzun metinli sahnelerde sıkışma olabilir.
- Risk: Ses (Chatterbox → ElevenLabs) geçişi izleyici tarafından fark edilebilir; "Egemen" sesi ile tutarlılık testi gerekli.

**Skor:** 8/10. **Öneriler:** (1) Shorts için ayrı tipografi ölçeği; (2) ses geçişinde A/B dinleme testi (ilk 40 sn ses örneği zaten teslimde var — iyi); (3) kapak A/B testi altyapısı.

---

## 6. Ajan 6 — Maliyet & Verimlilik Analisti

**Kapsam:** TTS kredisi, render süresi, kalite filtresi maliyeti, insan zamanı.

**Bulgular:**
- ElevenLabs karakter başına kredi → senaryo "az karakterli" yazma kuralı doğru maliyet optimizasyonu.
- Kalite filtresi 3 tur yeşil kapısı + V003'te 6 tur → TTS yeniden üretimi ve render tekrarı maliyetli; "sorunlu cümleye yalnız yeniden üretim" kuralı (V004 notu) doğru.
- Risk: Yerel GPU render maliyeti (elektrik/süre) ölçülmemiş; uzun video 8–13 dk + Shorts + kapaklar.
- Risk: `manifest.py` cümle bazlı durum takibi var ama maliyet (kredi, süre) boyutu yok.

**Skor:** 7/10. **Öneriler:** (1) Maliyet panosu: video başına TTS karakteri, render süresi, kalite tur sayısı; (2) kalite filtresinde "kırmızı" bulgu kök neden → kalıcı kural dönüşümü zaten var, maliyeti düşürür; (3) Shorts render'ı uzun videodan türetimde yeniden render yerine kırpma değerlendir.

---

## 7. Ajan 7 — Risk & Uyumluluk Analisti

**Kapsam:** Kaynak disiplini, politika, telif, yatırım tavsiyesi yasağı.

**Bulgular:**
- Her rakam kaynaklı (KAYNAKLAR.md, [K#]) + tahmin rozeti "≈ TAHMİN" + seste "tahmin" → şeffaflık ve telif riski düşük.
- "Uydurma insan personası yok; yatırım tavsiyesi yok; tıklama tuzağı yok" → YouTube politikası ve güvenilirlik için güçlü.
- Risk: Kaynak doğrulama "resmi/çapraz doğrulanmış" olarak tanımlı ama doğrulama süreci (kim, nasıl) dokümante edilmemiş; tek AI'ya güven.
- Risk: Ekonomi konularında (garantiler, köprü) iddialar politik hassasiyet taşıyabilir; "açıklamanın videodan kesin konuşmaması" kuralı (Kalite Filtresi C) doğru ama yeterli mi?
- Risk: Müzik/SFX kütüphanesi (`kutuphane/`) telif durumu belirsiz; KUTUPHANE.md'de lisans bilgisi olmalı.

**Skor:** 7,5/10. **Öneriler:** (1) KAYNAKLAR.md'ye doğrulama yöntemi (birincil/ikincil) ve tarih sütunu; (2) müzik/SFX lisans envanteri; (3) hassas konularda "iddia seviyesi" (kesin/olası/tahmin) etiketi.

---

## 8. Ajan 8 — Operasyon & Ölçeklenebilirlik Analisti

**Kapsam:** Faz sınırları, oturum yönetimi, dosya bütçeleri, teslim.

**Bulgular:**
- Faz sınırı (tek faz/oturum, ≈80k token veya 2000 satır) → bağlam taşması önlenir; DURUM.md ≤100 satır, bellek ≤200 satır → disiplinli.
- Teslim paketi eksiksiz (mp4, Shorts, kapak A/B, açıklama, SRT, ilk 40 sn ses, YUKLEME_PAKETI, SHA256) + paket_kontrol/bütünlük → profesyonel.
- Risk: "Kesin faz sınırı" kuralı AI'nın kendi kendini sınırlamasına dayalı; ihlal durumunda toparlama mekanizması yok.
- Risk: Tek repo/tek makine (Windows, venv, CUDA) → donanım bağımlılığı; yedekleme/DR planı yok.
- Risk: `uretim/` ve `teslim/` klasörleri büyüdükçe disk/versiyon yönetimi (git?) belirsiz.

**Skor:** 7,5/10. **Öneriler:** (1) Faz sonu otomatik DURUM.md güncelleme kontrolü (CI benzeri); (2) yedekleme stratejisi (bulut sync); (3) git ile uretim/teslim versiyonlama (büyük dosyalar için LFS).

---

## 9. Ajan 9 — Veri & Analitik Analisti

**Kapsam:** Ölçüm, geri bildirim döngüsü, A/B, kanal sesi.

**Bulgular:**
- ANALYTICS_LOG.md ve FEEDBACK.md var → geri bildirim altyapısı düşünülmüş.
- Ancak V001–V003 yayınlanmış/teslim edilmiş; ölçülebilir performans verisi (CTR, retention, watch time) henüz olgunlaşmamış → stratejik kararlar (sütun önceliği, konu seçimi) veri yerine sezgiye dayalı.
- Kalite filtresi metrikleri (CER, telaffuz, parlaklık sıçraması) teknik; izleyici metrikleri (retention drop-off) ile köprü kurulmamış.
- Risk: "3 tur yeşil" kapısı içerik kalitesini garanti eder ama izleyici davranışını tahmin etmez.

**Skor:** 6,5/10 (en düşük). **Öneriler:** (1) Video başına retention eğrisi + drop-off noktası → sahne bazlı korelasyon; (2) konu/sütun performans panosu; (3) Shorts → uzun video dönüşüm oranı ölçümü.

---

## 10. Ajan 10 — İnovasyon & Gelecek Yol Haritası Analisti

**Kapsam:** Yeni yetenekler, otomasyon, farklılaşma.

**Bulgular:**
- Mevcut farklaşma: kaynaklı veri + kodla çizim + sese kilitli görüntü + termal fiş imzası → güçlü niş.
- Potansiyel: çok dilli (İngilizce) versiyon, interaktif web (hesap fişi hesaplayıcı), haber/veri güncellemesiyle video yenileme (evergreen).
- Risk: Tüm üretim AI'da; "AI görüntü üretimi yok" kuralı farklılaştırıcı ama render maliyetini artırır; gelecekte hibrit (AI destekli şablon) değerlendirilebilir.
- Risk: Konsept kilitli (10. videodan sonra değişiklik) → inovasyon penceresi dar; erken deneyler (Shorts formatı, kapak stili) ile sınırlı.

**Skor:** 7,5/10. **Öneriler:** (1) 10. video öncesi "mikro deneyler" (kapak stili, Shorts hook) için izin; (2) web tabanlı "Hesap Fişi" hesaplayıcı (kanal web sitesi); (3) konu güncellemesi pipeline'ı (veri değişince video yenileme).

---

## 11. Sentez — Tek Birleşik Analiz

### 11.1 Konsensüs (ajanların ortak bulunduğu noktalar)
1. **Mimari tutarlı ve disiplinli.** Fazlı akış, tek-faz/oturum kuralı, bellek bütçeleri, kalite kapısı → sürdürülebilir bir üretim sistemi.
2. **Görsel-ses senkronu (zamanlama.json + ara()) projenin teknik kalbi.** Farklaşmayı sağlayan asıl mekanizma.
3. **Kaynak disiplini ve "tahmin" rozeti** güvenilirlik ve politika riskini minimize ediyor.
4. **Kalite filtresi (3 tur yeşil)** içerik kalitesini garanti ediyor ama maliyetli; V003'te 6 tur gerekmiş.
5. **Kullanıcı tek onay/darboğaz noktası** → ölçeklenebilirlik riski.

### 11.2 Çelişkiler ve dengeler
- **Kalite vs. Hız:** Ajan 3 ve 6 kalite kapısını maliyetli buluyor; Ajan 1 ve 4 hacim/büyüme istiyor. Denge: kapıyı sayısal rubrik ile hafiflet, "mikro deneyler" ile erken öğrenme.
- **Disiplin vs. Esneklik:** Ajan 1 ve 10 konsept kilidini erken aşamada kısıtlayıcı buluyor; Ajan 7 ve 8 disiplini risk azaltıcı buluyor. Denge: 10. video öncesi yalnız düşük riskli deneyler (kapak, Shorts hook) izin ver.
- **AI-üretim vs. İnsan değeri:** Ajan 7 ve 9 insan yargısının (retention, hassasiyet) eksikliğini vurguluyor; Ajan 2 ve 6 otomasyonu övüyor. Denge: teknik metrikler + izleyici metrikleri köprüsü.

### 11.3 Öncelikli Eylem Listesi (etki × maliyet)
| # | Eylem | Etki | Maliyet | Sahip ajan |
|---|---|---|---|---|
| 1 | ANALYTICS_LOG şablonu + video başına retention/drop-off ölçümü | Yüksek | Düşük | A9 |
| 2 | Kalite filtresini sayısal rubrik (0–3) ile puanla | Orta | Düşük | A3 |
| 3 | TTS soyut arayüzü (Chatterbox/ElevenLabs değiştirilebilir) | Orta | Orta | A2 |
| 4 | `zamanlama.json` şema doğrulama adımı | Orta | Düşük | A2 |
| 5 | Minimum haftalık 1 video taahhüdü + Shorts hunisi | Orta | Düşük | A4 |
| 6 | Maliyet panosu (TTS karakter, render süresi, tur sayısı) | Orta | Düşük | A6 |
| 7 | Müzik/SFX lisans envanteri + kaynak doğrulama yöntemi | Orta | Düşük | A7 |
| 8 | Yedekleme + git/LFS stratejisi | Orta | Orta | A8 |
| 9 | Shorts ayrı tipografi ölçeği | Düşük | Düşük | A5 |
| 10 | Web "Hesap Fişi" hesaplayıcı (10. video sonrası) | Yüksek | Yüksek | A10 |

### 11.4 Genel Verdict
Proje, **teknik ve editöryal olarak olgun, tutarlı ve kendi kendini iyileştiren** bir AI video üretim sistemidir. En büyük boşluk **ölçüm (analytics) katmanı** ve **kullanıcı onay darboğazı**dır; bunlar giderilmeden büyüme ve sürdürülebilirlik sınırlı kalır. Önerilen ilk adım: **her yayınlanan video için 7/30/90 gün metrik toplama + retention drop-off → sahne korelasyonu** (düşük maliyet, yüksek etki). Bu, konsept kilidinin (10. video) açılması için gerekli veriyi de üretecektir.

---

## 12. Notlar ve Sınırlılıklar
- Bu analiz brifing metnine dayanır; depoda kod bulunmadığından kod-seviyesi bulgular mimari çıkarımdır.
- Ajanlar simüle edilmiş uzman perspektiflerdir; her biri bağımsız bir "değerlendirme lensi" olarak çalıştırılmıştır.
- Güncel durum için `DURUM.md` ve `CHANNEL_STATE.md` geçerlidir (brifing §1).
