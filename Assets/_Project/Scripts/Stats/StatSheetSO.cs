using System;
using System.Collections.Generic;
using UnityEngine;

namespace Roguelite.Stats
{
    /// <summary>
    /// Bir karakterin/düşmanın BAŞLANGIÇ statları.
    /// Oluştur: Project > Create > Roguelite > Stats > Stat Sheet
    /// Her karakter (ör. "Knight", "Mage") ve her düşman türü için ayrı asset.
    /// Listede olmayan statlar StatDefaults'taki varsayılanları kullanır.
    /// </summary>
    [CreateAssetMenu(fileName = "StatSheet_New", menuName = "Roguelite/Stats/Stat Sheet")]
    public sealed class StatSheetSO : ScriptableObject
    {
        [Serializable]
        public struct Entry
        {
            public StatType Stat;
            public float BaseValue;
            [Tooltip("Sınır kullanılsın mı? Kapalıysa StatDefaults'taki sınırlar geçerli.")]
            public bool OverrideLimits;
            public float Min;
            public float Max;
        }

        [SerializeField] private List<Entry> entries = new List<Entry>();

        public IReadOnlyList<Entry> Entries => entries;

        public bool TryGetEntry(StatType stat, out Entry entry)
        {
            for (int i = 0; i < entries.Count; i++)
            {
                if (entries[i].Stat == stat)
                {
                    entry = entries[i];
                    return true;
                }
            }
            entry = default;
            return false;
        }

#if UNITY_EDITOR
        // Asset yaratıldığında (veya sağ tık > Reset) oyuncu için mantıklı değerlerle doldur.
        private void Reset()
        {
            entries.Clear();
            foreach (StatType t in Enum.GetValues(typeof(StatType)))
            {
                StatDefaults.Get(t, out float value, out _, out _);
                entries.Add(new Entry { Stat = t, BaseValue = value });
            }
        }
#endif
    }

    /// <summary>Sheet'te tanımlanmamış statlar için kod içi varsayılanlar ve sınırlar.</summary>
    public static class StatDefaults
    {
        public static void Get(StatType stat, out float value, out float min, out float max)
        {
            min = 0f;
            max = float.MaxValue;
            switch (stat)
            {
                case StatType.MaxHealth: value = 100f; min = 1f; break;
                case StatType.HealthRegen: value = 0f; break;
                case StatType.Armor: value = 0f; break;
                case StatType.MoveSpeed: value = 5f; min = 0.5f; max = 20f; break;
                case StatType.DashDistance: value = 3.2f; min = 0.5f; max = 12f; break;
                case StatType.DashCooldown: value = 1.2f; min = 0.1f; break;
                case StatType.DashCharges: value = 1f; min = 1f; max = 5f; break;
                case StatType.Might: value = 1f; break;
                case StatType.AttackSpeed: value = 1f; min = 0.1f; break;
                case StatType.CritChance: value = 0.05f; max = 1f; break;
                case StatType.CritMultiplier: value = 2f; min = 1f; break;
                case StatType.Area: value = 1f; min = 0.1f; break;
                case StatType.CooldownReduction: value = 0f; max = 1f; break;
                case StatType.ProjectileCount: value = 0f; break;
                case StatType.ProjectileSpeed: value = 1f; min = 0.1f; break;
                case StatType.Duration: value = 1f; min = 0.1f; break;
                case StatType.Knockback: value = 1f; break;
                case StatType.PickupRadius: value = 2f; min = 0.5f; break;
                case StatType.XpGain: value = 1f; break;
                case StatType.GoldGain: value = 1f; break;
                case StatType.Luck: value = 0f; break;
                default: value = 0f; break;
            }
        }
    }
}
