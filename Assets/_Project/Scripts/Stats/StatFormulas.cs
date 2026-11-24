using UnityEngine;

namespace Roguelite.Stats
{
    /// <summary>
    /// Statların oyun kuralına dönüştüğü TEK yer. Denge değişikliği = sadece burayı düzenle.
    /// </summary>
    public static class StatFormulas
    {
        public const float MaxCooldownReduction = 0.8f;
        public const float MinCooldown = 0.05f;

        /// <summary>CDR uygulanmış bekleme süresi. %80 CDR tavanı ile "sonsuz spam" engellenir.</summary>
        public static float Cooldown(float baseCooldown, float cooldownReduction)
        {
            float cdr = Mathf.Clamp(cooldownReduction, 0f, MaxCooldownReduction);
            return Mathf.Max(MinCooldown, baseCooldown * (1f - cdr));
        }

        /// <summary>Kritik zarı. Luck ileride buraya eklenebilir.</summary>
        public static bool RollCrit(float critChance) => Random.value < critChance;

        /// <summary>Armor için azalan getiri: 10 armor ≈ %33, 20 armor = %50 azaltma.</summary>
        public static float ApplyArmor(float damage, float armor)
        {
            if (armor <= 0f) return damage;
            return damage * (20f / (20f + armor));
        }
    }
}
