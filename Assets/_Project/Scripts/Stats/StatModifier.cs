using System;
using UnityEngine;

namespace Roguelite.Stats
{
    /// <summary>
    /// Hesaplama formülü (sıradan bağımsız, deterministik):
    ///
    ///   Final = (Base + ΣFlat) × (1 + ΣPercentAdd) × Π(1 + PercentMult)
    ///
    ///  - Flat        : +10 Can, +0.05 Kritik şansı
    ///  - PercentAdd  : "+%20 Hasar" — aynı türden olanlar TOPLANIR (%20 + %20 = %40). Eşyaların çoğu bunu kullanmalı.
    ///  - PercentMult : "x1.5 Hasar" (Value = 0.5) — birbirini ÇARPAR. Nadir/efsanevi eşyalar ve sinerjiler için.
    /// </summary>
    public enum StatModType
    {
        Flat = 100,
        PercentAdd = 200,
        PercentMult = 300,
    }

    /// <summary>
    /// Çalışma zamanı modifier'ı. Source = bu modifier'ı kim ekledi (eşya SO'su, sinerji, meta profili, buff...).
    /// Source sayesinde "bu eşyanın tüm etkilerini kaldır" tek satırdır: stats.RemoveAllModifiersFromSource(item).
    /// </summary>
    public sealed class StatModifier
    {
        public readonly float Value;
        public readonly StatModType Type;
        public readonly object Source;

        public StatModifier(float value, StatModType type, object source)
        {
            Value = value;
            Type = type;
            Source = source;
        }
    }

    /// <summary>
    /// Inspector'da düzenlenebilen modifier tanımı. Eşya, rün, sinerji ve meta yükseltme SO'ları bunun listesini tutar.
    /// </summary>
    [Serializable]
    public struct StatModifierData
    {
        public StatType Stat;
        public StatModType Type;
        [Tooltip("Flat: düz değer. PercentAdd/PercentMult: 0.2 = %20")]
        public float Value;

        public StatModifierData(StatType stat, StatModType type, float value)
        {
            Stat = stat;
            Type = type;
            Value = value;
        }

        public StatModifier ToModifier(object source, float scale = 1f) => new StatModifier(Value * scale, Type, source);

        public override string ToString()
        {
            switch (Type)
            {
                case StatModType.Flat: return $"{(Value >= 0 ? "+" : "")}{Value:0.##} {Stat}";
                case StatModType.PercentAdd: return $"{(Value >= 0 ? "+" : "")}{Value * 100f:0.#}% {Stat}";
                default: return $"x{1f + Value:0.##} {Stat}";
            }
        }
    }
}
