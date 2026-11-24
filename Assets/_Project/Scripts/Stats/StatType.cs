namespace Roguelite.Stats
{
    /// <summary>
    /// Tüm statlar. YENİ STAT EKLERKEN SONA EKLE (ScriptableObject'lerde enum int olarak serialize edilir,
    /// araya eklersen mevcut asset'lerin değerleri kayar).
    ///
    /// Yüzde statları 0..1 aralığında tutulur: CritChance 0.15 = %15.
    /// </summary>
    public enum StatType
    {
        // --- Hayatta kalma ---
        MaxHealth = 0,
        HealthRegen = 1,       // saniyede
        Armor = 2,             // düz hasar azaltma

        // --- Hareket ---
        MoveSpeed = 3,         // birim/sn
        DashDistance = 4,      // birim
        DashCooldown = 5,      // sn (CDR'den önce)
        DashCharges = 6,       // adet

        // --- Saldırı ---
        Might = 7,             // hasar çarpanı (1 = %100)
        AttackSpeed = 8,       // çarpan (1 = normal)
        CritChance = 9,        // 0..1
        CritMultiplier = 10,   // 2 = 2x hasar
        Area = 11,             // AoE / mermi boyutu çarpanı
        CooldownReduction = 12,// 0..1 (StatFormulas ile %80'de sınırlanır)
        ProjectileCount = 13,  // ek mermi adedi
        ProjectileSpeed = 14,  // çarpan
        Duration = 15,         // yetenek süresi çarpanı
        Knockback = 16,        // çarpan

        // --- Ekonomi / İlerleme ---
        PickupRadius = 17,     // mıknatıs yarıçapı (birim)
        XpGain = 18,           // çarpan
        GoldGain = 19,         // çarpan
        Luck = 20,             // nadir eşya / kritik rün şansı
    }
}
