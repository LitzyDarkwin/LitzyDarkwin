using System;
using System.Collections.Generic;
using UnityEngine;

namespace Roguelite.Stats
{
    /// <summary>
    /// Tek bir stat. Değer LAZY hesaplanır ve önbelleğe alınır (dirty flag):
    /// her frame 300 düşman Value okusa bile, modifier değişmedikçe hesap yapılmaz.
    /// </summary>
    public sealed class Stat
    {
        private readonly List<StatModifier> _modifiers = new List<StatModifier>(4);
        private float _baseValue;
        private float _cachedValue;
        private bool _isDirty = true;

        public readonly StatType Type;
        public float MinValue { get; private set; }
        public float MaxValue { get; private set; }

        /// <summary>(eski, yeni). Dinleyici varken stat "eager" olur ve değişim anında yayınlanır.</summary>
        public event Action<Stat, float, float> ValueChanged;

        public IReadOnlyList<StatModifier> Modifiers => _modifiers;

        public Stat(StatType type, float baseValue, float min = float.MinValue, float max = float.MaxValue)
        {
            Type = type;
            _baseValue = baseValue;
            MinValue = min;
            MaxValue = max;
        }

        public float BaseValue
        {
            get => _baseValue;
            set
            {
                float old = PeekForNotify();
                _baseValue = value;
                MarkDirtyAndNotify(old);
            }
        }

        public float Value
        {
            get
            {
                if (_isDirty)
                {
                    _cachedValue = Calculate();
                    _isDirty = false;
                }
                return _cachedValue;
            }
        }

        /// <summary>Int gerektiren statlar için (DashCharges, ProjectileCount).</summary>
        public int IntValue => Mathf.RoundToInt(Value);

        public void SetLimits(float min, float max)
        {
            float old = PeekForNotify();
            MinValue = min;
            MaxValue = max;
            MarkDirtyAndNotify(old);
        }

        public void AddModifier(StatModifier modifier)
        {
            if (modifier == null) return;
            float old = PeekForNotify();
            _modifiers.Add(modifier);
            MarkDirtyAndNotify(old);
        }

        public bool RemoveModifier(StatModifier modifier)
        {
            float old = PeekForNotify();
            if (!_modifiers.Remove(modifier)) return false;
            MarkDirtyAndNotify(old);
            return true;
        }

        public int RemoveAllModifiersFromSource(object source)
        {
            float old = PeekForNotify();
            int removed = 0;
            for (int i = _modifiers.Count - 1; i >= 0; i--)
            {
                if (ReferenceEquals(_modifiers[i].Source, source))
                {
                    _modifiers.RemoveAt(i);
                    removed++;
                }
            }
            if (removed > 0) MarkDirtyAndNotify(old);
            return removed;
        }

        public void ClearModifiers()
        {
            if (_modifiers.Count == 0) return;
            float old = PeekForNotify();
            _modifiers.Clear();
            MarkDirtyAndNotify(old);
        }

        private float Calculate()
        {
            float flat = 0f;
            float percentAdd = 0f;
            float multiplier = 1f;

            for (int i = 0; i < _modifiers.Count; i++)
            {
                StatModifier m = _modifiers[i];
                switch (m.Type)
                {
                    case StatModType.Flat: flat += m.Value; break;
                    case StatModType.PercentAdd: percentAdd += m.Value; break;
                    case StatModType.PercentMult: multiplier *= 1f + m.Value; break;
                }
            }

            float result = (_baseValue + flat) * Mathf.Max(0f, 1f + percentAdd) * multiplier;
            // float hassasiyet çöplerini temizle (0.30000001 -> 0.3)
            result = (float)Math.Round(result, 4);
            return Mathf.Clamp(result, MinValue, MaxValue);
        }

        private float PeekForNotify() => ValueChanged != null ? Value : 0f;

        private void MarkDirtyAndNotify(float oldValue)
        {
            _isDirty = true;
            if (ValueChanged == null) return;

            float newValue = Value;
            if (!Mathf.Approximately(oldValue, newValue))
                ValueChanged.Invoke(this, oldValue, newValue);
        }

        public override string ToString() => $"{Type}: {Value} (base {_baseValue}, {_modifiers.Count} mod)";
    }
}
