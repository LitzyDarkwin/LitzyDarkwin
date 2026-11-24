# ADIM 2 Kurulum Rehberi (Unity 2022.3 LTS veya Unity 6)

Gerekli paket yok. (DOTween opsiyonel; kullanmıyoruz, yaylar `Core/Utils/Spring.cs` ile yapılıyor.)
`Assets/_Project` klasörünü projenin `Assets/` klasörüne kopyalaman yeterli.

## ⚡ Hızlı Yol (önerilen): Tek Tıkla Kurulum
1. `Assets/_Project` klasörünü projene kopyala, derlemenin bitmesini bekle.
2. Üst menüden **Roguelite > Demo Sahnesini Kur**.
3. **Play**. (Aşağıdaki manuel adımların hepsini bu menü yapar.)

---

## 1. Katmanlar ve Fizik
- **Project Settings > Tags and Layers**: `Player`, `Enemy`, `EnemyProjectile`, `Pickup` katmanlarını ekle.
- **Physics 2D**: Gravity = (0, 0).
- **Time**: Fixed Timestep = 0.0166 (60 Hz) mobilde akıcı dash için önerilir.

## 2. ScriptableObject Asset'leri (Project penceresi > sağ tık > Create > Roguelite)
| Asset | Menü | Not |
|---|---|---|
| `InputChannel` | Input > Input Channel | Projede tek tane |
| `StatSheet_Player` | Stats > Stat Sheet | Oluşunca tüm statlar varsayılanlarla dolar |
| `MetaProgressionProfile` | Meta > Meta Progression Profile | Opsiyonel |
| `Meta_Might` vb. | Meta > Meta Upgrade | Opsiyonel, profile ekle |

## 3. Sahne Hiyerarşisi

```
[Systems]                      PoolManager, KeyboardInputBridge(InputChannel)
Player   (Layer: Player)       Rigidbody2D, CircleCollider2D(r=0.35), CharacterStats, PlayerController, DashGhostTrail
 ├─ Shadow                     SpriteRenderer (yarı saydam elips)
 └─ Visual                     ProceduralJuiceAnimator
     ├─ Sprite                 SpriteRenderer (pivot Bottom, ya da localPosition y=+0.5)
     └─ Trail (opsiyonel)      TrailRenderer (Emitting KAPALI, Time 0.15, Width 0.4→0)
Canvas                         Screen Space Overlay, CanvasScaler (Scale With Screen Size, 1080x1920, Match 0.5)
 ├─ JoystickArea               Stretch-full RectTransform, Image(alpha 0, Raycast ON), FloatingJoystick
 │   └─ Background             Image (halka) ~220px, Raycast OFF
 │       └─ Handle             Image (daire) ~100px, Raycast OFF
 └─ DashButton                 Sağ alt, ~200px, Image(Raycast ON), DashButton
     ├─ Icon                   Image
     ├─ CooldownFill           Image (Filled / Radial360, siyah %60), Raycast OFF
     └─ ChargesText            Text (opsiyonel)
EventSystem                    Yeni Input System kullanıyorsan "InputSystemUIInputModule"
```

## 4. Bileşen Ayarları
- **CharacterStats** (Player): Base Sheet = `StatSheet_Player`, Meta Progression = profil, **Broadcast Changes = ✔**
- **PlayerController**: Input Channel = `InputChannel`, **Pass Through Layers = Enemy, EnemyProjectile**
- **DashGhostTrail**: Source Renderer = `Visual/Sprite`, Ghost Prefab = `GhostSprite`, Optional Trail = `Visual/Trail`
- **GhostSprite prefab**: Boş obje + SpriteRenderer + `GhostSprite` → `Prefabs/FX/` altına prefab yap
- **FloatingJoystick**: Input Channel, Background, Handle
- **DashButton**: Input Channel, Icon, Cooldown Fill, (Charges Text)

## 5. Test
Play'e bas: WASD/oklar = hareket, Space/Shift = dash. Mobil simülasyon için Device Simulator'da fare ile joystick.
Player'a sağ tık > **Debug/Log All Stats** ile stat dökümünü görebilirsin.

## Hissi İnce Ayar Hızlı Tablo
| İstek | Değiştir |
|---|---|
| Daha "snappy" hareket | PlayerController.accelerationSharpness ↑ (25-30) |
| Bıraktıktan sonra biraz kayma | decelerationSharpness ↓ (8-10) |
| Daha jöle gibi karakter | Juice.scaleSpring.Damping ↓ (0.2), Frequency ↓ (4) |
| Daha sert dash punch | Juice.dashStretch ↑ (0.6) |
| Daha uzun ghost izi | DashGhostTrail.ghostLifetime ↑, spawnInterval ↓ |
