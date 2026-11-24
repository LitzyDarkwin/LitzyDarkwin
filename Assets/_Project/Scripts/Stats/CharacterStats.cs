using System;
using System.Collections.Generic;
using Roguelite.Core.Events;
using UnityEngine;

namespace Roguelite.Stats
{
    /// <summary>
    /// GameObject: Player kök objesine (ve ileride her düşman prefab'ına) ekle.
    ///
    /// Statlara erişim (enum index'li dizi -> Dictionary'den hızlı, GC yok):
    ///   float speed = stats[StatType.MoveSpeed];
    ///   stats.AddModifier(StatType.Might, new StatModifier(0.2f, StatModType.PercentAdd, itemSO));
    ///   stats.RemoveAllModifiersFromSource(itemSO);   // eşya çıkarılınca
    ///
    /// Katmanlar: StatSheet (base) -> MetaProgression (kalıcı) -> Eşyalar/Rünler/Sinerjiler/Buff'lar (run içi).
    /// </summary>
    [DefaultExecutionOrder(-500)] // PlayerController vb. Awake'lerinden önce hazır olsun
    [DisallowMultipleComponent]
    public sealed class CharacterStats : MonoBehaviour
    {
        [SerializeField] private StatSheetSO baseSheet;

        [Tooltip("Sadece oyuncu için atanır. Düşmanlarda boş bırak.")]
        [SerializeField] private MetaProgressionSO metaProgression;

        [Tooltip("Stat değişimlerini EventBus'a yayınla (HUD vb. için). Sadece OYUNCUDA aç; 300 düşmanda kapalı kalsın.")]
        [SerializeField] private bool broadcastChanges;

        private static readonly int StatCount = ComputeStatCount();
        private Stat[] _stats;
        private bool _initialized;

        /// <summary>Yerel dinleyiciler için (aynı GameObject). Sadece broadcastChanges açıkken tetiklenir.</summary>
        public event Action<StatType, float, float> StatChanged;

        public bool IsInitialized => _initialized;

        public float this[StatType type] => Get(type).Value;

        private void Awake()
        {
            if (!_initialized) Initialize(baseSheet);
        }

        /// <summary>
        /// Havuzlanan düşmanlar farklı sheet'lerle yeniden kullanılabilsin diye public.
        /// Tüm modifier'ları temizler ve sıfırdan kurar.
        /// </summary>
        public void Initialize(StatSheetSO sheet)
        {
            baseSheet = sheet;

            if (_stats == null)
            {
                _stats = new Stat[StatCount];
                for (int i = 0; i < StatCount; i++)
                {
                    StatType type = (StatType)i;
                    StatDefaults.Get(type, out float value, out float min, out float max);
                    _stats[i] = new Stat(type, value, min, max);

                    if (broadcastChanges)
                        _stats[i].ValueChanged += HandleStatValueChanged;
                }
            }
            else
            {
                for (int i = 0; i < StatCount; i++)
                {
                    StatDefaults.Get((StatType)i, out float value, out float min, out float max);
                    _stats[i].ClearModifiers();
                    _stats[i].SetLimits(min, max);
                    _stats[i].BaseValue = value;
                }
            }

            if (sheet != null)
            {
                IReadOnlyList<StatSheetSO.Entry> entries = sheet.Entries;
                for (int i = 0; i < entries.Count; i++)
                {
                    StatSheetSO.Entry e = entries[i];
                    Stat stat = Get(e.Stat);
                    if (e.OverrideLimits) stat.SetLimits(e.Min, e.Max);
                    stat.BaseValue = e.BaseValue;
                }
            }

            if (metaProgression != null)
                metaProgression.ApplyTo(this);

            _initialized = true;
        }

        public Stat Get(StatType type)
        {
            if (!_initialized) Initialize(baseSheet);
            return _stats[(int)type];
        }

        public float GetValue(StatType type) => Get(type).Value;
        public int GetInt(StatType type) => Get(type).IntValue;

        public void AddModifier(StatType type, StatModifier modifier) => Get(type).AddModifier(modifier);

        public bool RemoveModifier(StatType type, StatModifier modifier) => Get(type).RemoveModifier(modifier);

        /// <summary>Bir eşyanın/rünün/sinerjinin tüm modifier listesini tek seferde uygular.</summary>
        public void AddModifiers(IReadOnlyList<StatModifierData> modifiers, object source, float scale = 1f)
        {
            for (int i = 0; i < modifiers.Count; i++)
            {
                StatModifierData d = modifiers[i];
                Get(d.Stat).AddModifier(d.ToModifier(source, scale));
            }
        }

        /// <summary>Source'a ait her şeyi tüm statlardan kaldırır (eşya satıldı, buff bitti, sinerji kırıldı).</summary>
        public int RemoveAllModifiersFromSource(object source)
        {
            if (!_initialized) return 0;
            int total = 0;
            for (int i = 0; i < _stats.Length; i++)
                total += _stats[i].RemoveAllModifiersFromSource(source);
            return total;
        }

        private void HandleStatValueChanged(Stat stat, float oldValue, float newValue)
        {
            StatChanged?.Invoke(stat.Type, oldValue, newValue);
            EventBus<StatChangedEvent>.Raise(new StatChangedEvent
            {
                Owner = this,
                Stat = stat.Type,
                OldValue = oldValue,
                NewValue = newValue
            });
        }

        private static int ComputeStatCount()
        {
            int max = 0;
            foreach (int v in Enum.GetValues(typeof(StatType)))
                if (v > max) max = v;
            return max + 1;
        }

#if UNITY_EDITOR
        [ContextMenu("Debug/Log All Stats")]
        private void LogAllStats()
        {
            if (!_initialized) Initialize(baseSheet);
            var sb = new System.Text.StringBuilder($"[{name}] Stats\n");
            for (int i = 0; i < _stats.Length; i++) sb.AppendLine(_stats[i].ToString());
            Debug.Log(sb.ToString(), this);
        }
#endif
    }
}
