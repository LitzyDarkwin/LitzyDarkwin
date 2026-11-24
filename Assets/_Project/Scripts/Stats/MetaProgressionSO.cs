using System;
using System.Collections.Generic;
using UnityEngine;

namespace Roguelite.Stats
{
    /// <summary>
    /// Oyuncunun kalıcı yükseltme profili. CharacterStats Awake'te bunu okuyup modifier olarak uygular.
    /// Oluştur: Create > Roguelite > Meta > Meta Progression Profile
    ///
    /// Not: Seviyeler şimdilik Inspector'dan/PlayerPrefs'ten gelir. Kayıt sistemi (JSON) eklendiğinde sadece
    /// Load/Save metodları değişir, stat tarafı hiç değişmez.
    /// </summary>
    [CreateAssetMenu(fileName = "MetaProgressionProfile", menuName = "Roguelite/Meta/Meta Progression Profile")]
    public sealed class MetaProgressionSO : ScriptableObject
    {
        [Serializable]
        public struct UpgradeLevel
        {
            public MetaUpgradeSO Upgrade;
            [Min(0)] public int Level;
        }

        [SerializeField] private List<UpgradeLevel> upgrades = new List<UpgradeLevel>();
        [SerializeField] private bool loadFromPlayerPrefs = true;

        private const string PrefsPrefix = "meta_lvl_";

        public int GetLevel(MetaUpgradeSO upgrade)
        {
            if (upgrade == null) return 0;
            if (loadFromPlayerPrefs && PlayerPrefs.HasKey(PrefsPrefix + upgrade.Id))
                return Mathf.Clamp(PlayerPrefs.GetInt(PrefsPrefix + upgrade.Id), 0, upgrade.MaxLevel);

            for (int i = 0; i < upgrades.Count; i++)
                if (upgrades[i].Upgrade == upgrade)
                    return Mathf.Clamp(upgrades[i].Level, 0, upgrade.MaxLevel);
            return 0;
        }

        public void SetLevel(MetaUpgradeSO upgrade, int level)
        {
            PlayerPrefs.SetInt(PrefsPrefix + upgrade.Id, Mathf.Clamp(level, 0, upgrade.MaxLevel));
            PlayerPrefs.Save();
        }

        /// <summary>Tüm kalıcı modifier'ları hedef stat konteynerine uygular. Source = bu profil.</summary>
        public void ApplyTo(CharacterStats stats)
        {
            for (int i = 0; i < upgrades.Count; i++)
            {
                MetaUpgradeSO up = upgrades[i].Upgrade;
                if (up == null) continue;

                int level = GetLevel(up);
                if (level <= 0) continue;

                for (int m = 0; m < up.ModifiersPerLevel.Count; m++)
                {
                    StatModifierData data = up.ModifiersPerLevel[m];
                    stats.AddModifier(data.Stat, data.ToModifier(this, level));
                }
            }
        }
    }
}
