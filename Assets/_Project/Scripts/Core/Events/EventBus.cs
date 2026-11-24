using System;
using System.Collections.Generic;
using UnityEngine;

namespace Roguelite.Core.Events
{
    /// <summary>
    /// Tüm global oyun olaylarının işaretleyici arayüzü.
    /// Olaylar daima STRUCT olmalıdır -> Raise sırasında heap allocation (GC) olmaz.
    /// </summary>
    public interface IGameEvent { }

    /// <summary>
    /// Tip-güvenli, sıfır-GC, statik Event Bus.
    ///
    /// Kullanım:
    ///   EventBus&lt;EnemyKilledEvent&gt;.Subscribe(OnEnemyKilled);     // OnEnable
    ///   EventBus&lt;EnemyKilledEvent&gt;.Unsubscribe(OnEnemyKilled);   // OnDisable
    ///   EventBus&lt;EnemyKilledEvent&gt;.Raise(new EnemyKilledEvent { ... });
    ///
    /// Kural: Aynı GameObject üzerindeki bileşenler birbirleriyle doğrudan C# event'leri ile konuşur
    /// (ör. PlayerController -> ProceduralJuiceAnimator). Sistemler ARASI (Player -> UI, Enemy -> XP,
    /// Inventory -> Synergy) iletişim EventBus üzerinden yapılır; böylece hiçbir sistem diğerini tanımak zorunda kalmaz.
    /// </summary>
    public static class EventBus<T> where T : struct, IGameEvent
    {
        private static Action<T> _handlers;

        static EventBus()
        {
            // Domain Reload kapalıyken (Enter Play Mode Options) eski aboneler sızmasın diye kayıt oluyoruz.
            EventBusRegistry.Register(Clear);
        }

        public static void Subscribe(Action<T> handler) => _handlers += handler;

        public static void Unsubscribe(Action<T> handler) => _handlers -= handler;

        public static void Raise(T evt) => _handlers?.Invoke(evt);

        public static bool HasListeners => _handlers != null;

        public static void Clear() => _handlers = null;
    }

    internal static class EventBusRegistry
    {
        private static readonly List<Action> Clearers = new List<Action>(32);

        public static void Register(Action clear) => Clearers.Add(clear);

        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.SubsystemRegistration)]
        private static void ClearAll()
        {
            for (int i = 0; i < Clearers.Count; i++)
                Clearers[i]?.Invoke();
        }
    }
}
