using System;
using UnityEngine;

namespace Roguelite.Core
{
    /// <summary>
    /// Hareket eden her şey (Oyuncu, Düşman) bunu uygular.
    /// ProceduralJuiceAnimator sadece bu arayüzü tanır -> Juice katmanı Player/Enemy koduna bağımlı değildir.
    /// </summary>
    public interface IVelocityProvider
    {
        Vector2 Velocity { get; }
        /// <summary>Normalizasyon için (wobble/squash şiddeti = hız / MaxSpeed).</summary>
        float MaxSpeed { get; }
    }

    /// <summary>Dash yapabilen varlıklar (oyuncu, ileride "charger" düşmanlar).</summary>
    public interface IDashEventSource
    {
        event Action<Vector2> DashStarted;
        event Action DashEnded;
        bool IsDashing { get; }
    }

    /// <summary>Hasar sistemi (Adım 3) hasar vermeden önce bunu sorar.</summary>
    public interface IInvulnerable
    {
        bool IsInvulnerable { get; }
    }
}
