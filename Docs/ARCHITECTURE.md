# Mimari: Action-Roguelite / Bullet Heaven (Unity, C#, Mobil)

## 1. Katmanlar ve Bağımlılık Yönü

```
            ┌──────────────────────────────────────────────┐
            │                   CORE                       │  EventBus, IGameEvent, arayüzler,
            │  (hiçbir oyun sistemini tanımaz)             │  Spring, uyumluluk yardımcıları
            └──────────────────────────────────────────────┘
                 ▲          ▲           ▲            ▲
     ┌───────────┘   ┌──────┘     ┌─────┘      ┌─────┘
  Pooling          Stats        Controls      Juice           ← "Altyapı" katmanı (birbirini tanımaz)
     ▲               ▲             ▲            ▲
     └───────┬───────┴──────┬──────┴─────┬──────┘
          Player        Combat/Enemies  Abilities(Runes)  Inventory(Synergy)  Progression   ← "Oyun" katmanı
                              ▲
                              UI  (sadece EventBus + SO kanallarını dinler)
```

**Kural:** Ok yönünde bağımlılık olur, tersine asla. `Juice` bir `PlayerController` görmez, sadece `IVelocityProvider` / `IDashEventSource` görür → aynı animatör düşmanlarda da çalışır.

## 2. Klasör Yapısı

```
Assets/_Project/
├── Scripts/
│   ├── Core/
│   │   ├── Events/        EventBus.cs, GameEvents.cs        (global olay sözleşmeleri)
│   │   ├── Interfaces/    CoreInterfaces.cs                 (IVelocityProvider, IDashEventSource, IInvulnerable)
│   │   └── Utils/         Spring.cs, Rigidbody2DExtensions.cs
│   ├── Pooling/           ObjectPool<T>, PrefabPool, PooledObject, PoolManager, IPoolable
│   ├── Stats/             StatType, StatModifier, Stat, CharacterStats, StatSheetSO,
│   │                      MetaUpgradeSO, MetaProgressionSO, StatFormulas
│   ├── Controls/          InputChannelSO, FloatingJoystick, DashButton, KeyboardInputBridge
│   ├── Player/            PlayerController
│   ├── Juice/             ProceduralJuiceAnimator, DashGhostTrail, GhostSprite
│   │                      (ADIM 3: HitFlash, CameraShake, DamageNumber, HitStop)
│   ├── Combat/            (ADIM 3) Health, DamageInfo, Hitbox, KnockbackReceiver
│   ├── Enemies/           (ADIM 3) Enemy, EnemySpawner, EnemyDefinitionSO
│   ├── Abilities/         (ADIM 3) AbilityDefinitionSO, AbilityRuntime, Projectile
│   │   └── Runes/         RuneSO (decorator), SplitRune, BounceRune, PoisonRune, CritExplodeRune
│   ├── Inventory/         (ADIM 3) ItemSO, ItemTagSO, SynergySO, InventoryManager, SynergyManager
│   ├── Progression/       (ADIM 3) XpSystem, Pickup (mıknatıs), LevelUpDraft
│   └── UI/                (ADIM 3) HUD, LevelUpPanel, DamageNumberView
├── ScriptableObjects/     Stats/, Meta/, Input/, Abilities/, Runes/, Items/, Tags/, Synergies/, Enemies/
├── Prefabs/               Player, Enemies/, Projectiles/, FX/ (GhostSprite, HitSpark...)
├── Art/  Audio/  Materials/  Scenes/
```

## 3. Haberleşme Modeli (Event-Driven)

Üç kanal kullanıyoruz; hangisinin ne zaman kullanılacağı nettir:

| Kanal | Ne zaman | Örnek |
|---|---|---|
| **C# event (yerel)** | Aynı GameObject/prefab içindeki bileşenler | `PlayerController.DashStarted` → `ProceduralJuiceAnimator`, `DashGhostTrail` |
| **EventBus\<T\> (global, struct)** | Birbirini tanımaması gereken sistemler | `EnemyKilledEvent` → XP, Ses, Kill sayacı, "öldürünce patla" rünü |
| **ScriptableObject kanal** | Sahne UI ↔ (sonradan spawn olan) prefab | `InputChannelSO`: Joystick yazar, Player okur |

`EventBus` olayları **struct** olduğundan `Raise` sırasında GC oluşmaz; abonelik `OnEnable`/`OnDisable`'da yapılır.

### Örnek akış: Düşman ölümü (ADIM 3'te kodlanacak)

```
Projectile ─hit─► Health.TakeDamage(DamageInfo)
                     ├─► (yerel) HitFlash, Knockback, JuiceAnimator.PunchScale
                     ├─► EventBus<DamageDealtEvent>  → DamageNumberSpawner (pool), CameraShake (crit ise), Rünler (zehir)
                     └─ ölürse EventBus<EnemyKilledEvent>
                              ├─► XpDropper → PoolManager.Spawn(xpGem)
                              ├─► SynergyManager ("[Ateş] x5: ölen düşman alev bırakır")
                              └─► PoolManager.Despawn(enemy)
```

### Örnek akış: Eşya alımı → Stat → Sinerji

```
LevelUpPanel ─seçim─► InventoryManager.Add(itemSO)
                          ├─ CharacterStats.AddModifiers(item.Modifiers, source: item)
                          └─ EventBus<ItemAcquiredEvent>
                                   └─► SynergyManager: tag sayımı (3/5 eşik) → CharacterStats.AddModifiers(synergy, source: synergy)
                                                                           → EventBus<SynergyActivatedEvent> → UI banner + ekran flaşı
CharacterStats ─(broadcast)─► EventBus<StatChangedEvent> → HUD, PickupMagnet yarıçapı, Dash butonu
```

## 4. Stat Formülü

```
Final = clamp( (Base + ΣFlat) × (1 + ΣPercentAdd) × Π(1 + PercentMult), Min, Max )
```
- **Base**: `StatSheetSO` (karakter başına)
- **Kalıcı**: `MetaProgressionSO` (source = profil)
- **Run içi**: eşya, rün, sinerji, buff (her biri kendi `source`'u ile → tek satırda geri alınır)
- Değer lazy + dirty-flag önbellekli; 300 düşman her frame okusa bile hesap yalnızca değişimde yapılır.

## 5. Performans Kuralları (60 FPS, 300+ nesne)

1. Oyun döngüsünde **`Instantiate`/`Destroy` yok** → `PoolManager.Spawn/Despawn`. Sahne açılışında `presets` ile prewarm.
2. Olaylar struct, `foreach` yerine `for`, LINQ yok, `Update`'te `GetComponent` yok, string birleştirme yok.
3. Spawn edilen nesneler sahne kökünde (tek parent altında yüzlerce hareketli transform = paralelleşmeyen güncelleme).
4. Juice = matematik (spring), Animator/iskelet yok. Uzaktaki düşmanlarda `enableWobble` kapatılabilir.
5. ADIM 3: Düşman hareketi için tek bir `EnemyManager.Update` (300 ayrı `Update` yerine), hit flash için `MaterialPropertyBlock` (materyal kopyalama yok), hasar yazıları için tek TMP atlas.

## 6. Genişletilebilirlik (Yeni içerik = yeni asset)

| Eklemek istediğin | Yapacağın |
|---|---|
| Yeni stat | `StatType` sonuna ekle, `StatDefaults`'a varsayılan yaz |
| Kalıcı yükseltme | Create > Roguelite > Meta > Meta Upgrade, profile ekle |
| Yeni karakter | Yeni `StatSheetSO` + sprite |
| Yeni rün / yetenek / eşya / sinerji (ADIM 3) | İlgili SO'yu oluştur; kod değişikliği yalnızca *yeni davranış tipi* için |
