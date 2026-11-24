using UnityEngine;

namespace Roguelite.Core.Events
{
    // =====================================================================
    //  GLOBAL OLAY SÖZLEŞMELERİ (Event Contracts)
    //  Tüm olaylar struct'tır (GC yok). Yeni sistem = yeni struct + Raise.
    // =====================================================================

    // ---------- OYUNCU / HAREKET ----------

    /// <summary>Dash başladığında. Kamera, ses, UI, rünler ("dash atınca patla") dinleyebilir.</summary>
    public struct PlayerDashStartedEvent : IGameEvent
    {
        public Vector2 Direction;
        public Vector3 Position;
    }

    public struct PlayerDashEndedEvent : IGameEvent
    {
        public Vector3 Position;
    }

    /// <summary>Dash şarjı harcandığında veya yenilendiğinde. Dash butonu UI'ı bunu dinler.</summary>
    public struct DashChargesChangedEvent : IGameEvent
    {
        public int Current;
        public int Max;
        /// <summary>Şu anki şarj dolumuna kalan süre (0 = dolum yok).</summary>
        public float RechargeRemaining;
        /// <summary>Tek bir şarjın toplam dolum süresi (CDR uygulanmış).</summary>
        public float RechargeDuration;
    }

    // ---------- STAT ----------

    /// <summary>
    /// Sadece "broadcastChanges" açık CharacterStats (genelde oyuncu) tarafından yayınlanır.
    /// HUD, dinamik pickup yarıçapı, kamera zoom (Area statı) vb. dinler.
    /// </summary>
    public struct StatChangedEvent : IGameEvent
    {
        public Stats.CharacterStats Owner;
        public Stats.StatType Stat;
        public float OldValue;
        public float NewValue;
    }

    // ---------- ADIM 3'te doldurulacak sözleşmeler (şimdiden tanımlı ki diğer sistemler derlensin) ----------

    public struct EnemyKilledEvent : IGameEvent
    {
        public Vector3 Position;
        public int XpValue;
        public bool WasCritical;
    }

    public struct PlayerLevelUpEvent : IGameEvent
    {
        public int NewLevel;
    }

    public struct PickupCollectedEvent : IGameEvent
    {
        public int PickupTypeId;  // XP, Gold, Heal... (Adım 3'te enum'a dönecek)
        public int Amount;
        /// <summary>Art arda toplama sayacı -> artan pitch için.</summary>
        public int ComboIndex;
    }
}
