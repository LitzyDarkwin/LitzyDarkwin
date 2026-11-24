using System;
using UnityEngine;

namespace Roguelite.Core.Utils
{
    /// <summary>
    /// Yay ayarı. Frequency (Hz) = saniyede kaç kez salınır (yüksek = sert/çabuk),
    /// Damping (0..1+) = 0 sonsuz zıplar, 1 hiç taşmadan oturur (critically damped). "Tatlı" his için 0.3-0.5.
    /// </summary>
    [Serializable]
    public struct SpringConfig
    {
        [Min(0.01f)] public float Frequency;
        [Range(0f, 2f)] public float Damping;

        public SpringConfig(float frequency, float damping)
        {
            Frequency = frequency;
            Damping = damping;
        }

        public float AngularFrequency => 2f * Mathf.PI * Frequency;
    }

    /// <summary>
    /// Sönümlü harmonik osilatör (damped spring). DOTween yerine kullanılır çünkü:
    ///  - Her an kesilebilir / üst üste binebilir (tween'ler çakışınca "pop" yapar, yay yapmaz)
    ///  - Sıfır GC, struct
    ///  - Sürekli değişen hedefleri (hız bazlı squash) doğal takip eder
    /// Kararlılık için sabit küçük alt-adımlarla (sub-step) entegre edilir.
    /// </summary>
    public struct SpringFloat
    {
        public float Value;
        public float Velocity;

        private const float MaxSubStep = 1f / 120f;
        private const float MaxDeltaTime = 0.1f; // lag spike'larında patlamasın

        public SpringFloat(float initial)
        {
            Value = initial;
            Velocity = 0f;
        }

        public void Reset(float value)
        {
            Value = value;
            Velocity = 0f;
        }

        public void Step(float target, in SpringConfig config, float deltaTime)
        {
            float omega = config.AngularFrequency;
            float omegaSq = omega * omega;
            float twoZetaOmega = 2f * config.Damping * omega;

            float remaining = Mathf.Min(deltaTime, MaxDeltaTime);
            while (remaining > 0f)
            {
                float h = remaining < MaxSubStep ? remaining : MaxSubStep;
                float accel = -twoZetaOmega * Velocity - omegaSq * (Value - target);
                Velocity += accel * h;     // semi-implicit Euler: önce hız,
                Value += Velocity * h;     // sonra konum -> kararlı
                remaining -= h;
            }
        }

        /// <summary>
        /// Yaklaşık "bu kadar sapma yarat" anlamında darbe verir (birim: Value cinsinden).
        /// Ör: AddDisplacementImpulse(0.3f) -> değer kabaca 0.3 kadar fırlar ve geri yaylanır.
        /// </summary>
        public void AddDisplacementImpulse(float displacement, in SpringConfig config)
        {
            Velocity += displacement * config.AngularFrequency;
        }
    }
}
