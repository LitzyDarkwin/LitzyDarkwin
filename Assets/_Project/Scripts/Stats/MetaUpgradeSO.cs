using System.Collections.Generic;
using UnityEngine;

namespace Roguelite.Stats
{
    /// <summary>
    /// Kalıcı (run'lar arası) tek bir yükseltme tanımı. Ör: "Güçlenme: Seviye başı +%5 Might, maks 5 seviye".
    /// Oluştur: Create > Roguelite > Meta > Meta Upgrade
    /// </summary>
    [CreateAssetMenu(fileName = "Meta_New", menuName = "Roguelite/Meta/Meta Upgrade")]
    public sealed class MetaUpgradeSO : ScriptableObject
    {
        [Tooltip("Kayıt sisteminde anahtar olarak kullanılır. Yayından sonra DEĞİŞTİRME.")]
        public string Id = "meta_might";
        public string DisplayName = "Güçlenme";
        [TextArea] public string Description;
        public Sprite Icon;

        [Tooltip("Seviye başına eklenecek modifier'lar (seviye ile çarpılır).")]
        public List<StatModifierData> ModifiersPerLevel = new List<StatModifierData>();

        [Min(1)] public int MaxLevel = 5;
        [Tooltip("Seviye i'ye çıkmanın altın bedeli (index 0 = 1. seviye).")]
        public int[] GoldCostPerLevel = { 100, 200, 400, 800, 1600 };
    }
}
