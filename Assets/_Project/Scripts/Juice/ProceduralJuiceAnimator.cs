using Roguelite.Core;
using Roguelite.Core.Utils;
using UnityEngine;

namespace Roguelite.Juice
{
    /// <summary>
    /// GameObject: Karakterin "Visual" ÇOCUK objesine ekle (Rigidbody/Collider'ın olduğu kök objeye DEĞİL).
    ///
    ///   Player                    (Rigidbody2D, Collider, CharacterStats, PlayerController, DashGhostTrail)
    ///    ├─ Shadow                (SpriteRenderer - sabit kalır, zıplamaz)
    ///    └─ Visual                &lt;-- BU SCRIPT (sadece transform; scale/rotation/position'ı bu script yönetir)
    ///         └─ Sprite           (SpriteRenderer, localPosition (0, 0.5) gibi -> pivot ayakta olsun)
    ///
    /// Not: Esneme ayaktan olsun diye sprite'ın pivotu "Bottom" olmalı ya da Sprite çocuğu yukarı kaydırılmalı.
    ///
    /// İskelet animasyonu olmadan "canlı" his üretir:
    ///  - Yürürken sağa-sola sallanma (wobble) + harekete doğru eğilme (lean) + adım zıplaması (bob)
    ///  - Her adımda squash &amp; stretch (hacim korunur: x = 1/y)
    ///  - Dururken nefes alma (idle breathing)
    ///  - Yön değiştirirken scale.x yay ile -1/+1 arası döner -> tatlı "dönüş" sıkışması
    ///  - Dash başında yön boyunca uzama, dash sonunda iniş ezilmesi (IDashEventSource varsa otomatik)
    ///  - Dışarıdan çağrılabilir: PunchScale / PunchRotation / Squash (hasar alma, level-up, toplama için)
    ///
    /// Düşmanlarda da aynen kullanılır (IVelocityProvider uygulayan Enemy ile). 300 düşman için: ayarlarda
    /// breathing'i kapatabilir, enableWobble'ı uzak düşmanlarda false yapabilirsin. Tamamen GC'sizdir.
    /// </summary>
    [DisallowMultipleComponent]
    public sealed class ProceduralJuiceAnimator : MonoBehaviour
    {
        [Header("Kaynak (boşsa parent'ta IVelocityProvider aranır)")]
        [SerializeField] private MonoBehaviour velocitySourceOverride;

        [Header("Yürüme")]
        [SerializeField] private bool enableWobble = true;
        [Tooltip("Max hızda saniyedeki adım döngüsü (1 döngü = sağ+sol adım).")]
        [SerializeField] private float stepCycleFrequency = 2.4f;
        [SerializeField] private float wobbleAngle = 7f;
        [Tooltip("Harekete doğru gövde eğimi (derece).")]
        [SerializeField] private float leanAngle = 6f;
        [SerializeField] private float bobHeight = 0.07f;
        [Tooltip("Adım başı squash/stretch oranı.")]
        [SerializeField] private float stepSquash = 0.07f;

        [Header("İvme Esnemesi")]
        [Tooltip("Ani hızlanma/yavaşlamada hareket yönünde uzama. 0 = kapalı.")]
        [SerializeField] private float accelerationStretch = 0.012f;
        [SerializeField] private float maxAccelerationStretch = 0.18f;

        [Header("Idle Nefes")]
        [SerializeField] private bool enableBreathing = true;
        [SerializeField] private float breatheFrequency = 1.3f;
        [SerializeField] private float breatheAmount = 0.035f;

        [Header("Yön Çevirme")]
        [SerializeField] private bool flipWithVelocity = true;
        [Tooltip("Sprite varsayılan olarak sola bakıyorsa işaretle.")]
        [SerializeField] private bool spriteFacesLeft;
        [Tooltip("Yön çevirmek için gereken yatay hız oranı (titremeyi önler).")]
        [SerializeField, Range(0f, 1f)] private float flipThreshold = 0.12f;

        [Header("Dash")]
        [SerializeField] private float dashStretch = 0.45f;
        [SerializeField] private float dashTilt = 14f;
        [SerializeField] private float dashEndSquash = 0.28f;

        [Header("Yaylar")]
        [SerializeField] private SpringConfig scaleSpring = new SpringConfig(5.5f, 0.32f);
        [SerializeField] private SpringConfig rotationSpring = new SpringConfig(4f, 0.45f);
        [SerializeField] private SpringConfig flipSpring = new SpringConfig(6.5f, 0.75f);
        [SerializeField] private SpringConfig bobSpring = new SpringConfig(8f, 0.6f);

        private IVelocityProvider _source;
        private IDashEventSource _dashSource;

        private Vector3 _baseScale;
        private Vector3 _basePosition;

        private SpringFloat _scaleX;
        private SpringFloat _scaleY;
        private SpringFloat _rotation;
        private SpringFloat _flip;
        private SpringFloat _bob;

        private float _phase;
        private float _breathPhase;
        private float _facing = 1f;
        private Vector2 _prevVelocity;

        public float Facing => _facing;

        private void Awake()
        {
            _baseScale = transform.localScale;
            _basePosition = transform.localPosition;

            _source = velocitySourceOverride as IVelocityProvider;
            if (_source == null) _source = GetComponentInParent<IVelocityProvider>();
            _dashSource = GetComponentInParent<IDashEventSource>();

            _breathPhase = Random.value * 10f; // kalabalık düşmanlar senkron nefes almasın
        }

        private void OnEnable()
        {
            // Havuzdan yeniden çıkınca temiz başla
            _scaleX.Reset(1f);
            _scaleY.Reset(1f);
            _rotation.Reset(0f);
            _bob.Reset(0f);
            _flip.Reset(_facing);
            _prevVelocity = Vector2.zero;
            _phase = 0f;

            if (_dashSource != null)
            {
                _dashSource.DashStarted += OnDashStarted;
                _dashSource.DashEnded += OnDashEnded;
            }
        }

        private void OnDisable()
        {
            if (_dashSource != null)
            {
                _dashSource.DashStarted -= OnDashStarted;
                _dashSource.DashEnded -= OnDashEnded;
            }
            transform.localScale = _baseScale;
            transform.localPosition = _basePosition;
            transform.localRotation = Quaternion.identity;
        }

        private void LateUpdate()
        {
            float dt = Time.deltaTime;
            if (dt <= 0f) return;

            Vector2 velocity = _source != null ? _source.Velocity : Vector2.zero;
            float maxSpeed = _source != null ? Mathf.Max(0.01f, _source.MaxSpeed) : 1f;
            float speed01 = Mathf.Clamp01(velocity.magnitude / maxSpeed);
            bool dashing = _dashSource != null && _dashSource.IsDashing;

            // ---------------- Adım döngüsü
            float wobble = 0f, bob01 = 0f;
            if (enableWobble && !dashing)
            {
                _phase += dt * stepCycleFrequency * speed01;
                if (_phase > 1000f) _phase -= 1000f;
                float s = Mathf.Sin(_phase * 2f * Mathf.PI);
                wobble = s;                 // -1..1 : sol-sağ
                bob01 = Mathf.Abs(s);       // 0..1  : her adımda bir zıplama
            }

            // ---------------- Hedef rotasyon: wobble + lean
            float targetRot = 0f;
            if (!dashing)
            {
                targetRot = wobble * wobbleAngle * speed01;
                targetRot -= (velocity.x / maxSpeed) * leanAngle; // sağa gidiyorsa sağa yatsın (z negatif)
            }

            // ---------------- Hedef ölçek: adım squash + nefes + ivme esnemesi
            float stretchY = (bob01 * 2f - 1f) * stepSquash * speed01; // havadayken uzun, inişte basık

            if (enableBreathing)
            {
                _breathPhase += dt * breatheFrequency;
                stretchY += Mathf.Sin(_breathPhase * 2f * Mathf.PI) * breatheAmount * (1f - speed01);
            }

            float targetX = 1f;
            float targetY = 1f + stretchY;

            if (accelerationStretch > 0f && !dashing)
            {
                Vector2 accel = (velocity - _prevVelocity) / dt;
                float ax = Mathf.Clamp(Mathf.Abs(accel.x) * accelerationStretch / maxSpeed, 0f, maxAccelerationStretch);
                float ay = Mathf.Clamp(Mathf.Abs(accel.y) * accelerationStretch / maxSpeed, 0f, maxAccelerationStretch);
                targetX += ax - ay * 0.5f;
                targetY += ay - ax * 0.5f;
            }
            _prevVelocity = velocity;

            // Hacim koruma (yaklaşık): biri uzarsa diğeri incelir
            float volume = targetX * targetY;
            if (volume > 0.0001f)
            {
                float k = 1f / Mathf.Sqrt(volume);
                targetX *= k;
                targetY *= k;
            }

            // ---------------- Yön
            if (flipWithVelocity && Mathf.Abs(velocity.x) > flipThreshold * maxSpeed)
                _facing = velocity.x > 0f ? 1f : -1f;

            // ---------------- Yayları ilerlet
            _scaleX.Step(targetX, scaleSpring, dt);
            _scaleY.Step(targetY, scaleSpring, dt);
            _rotation.Step(targetRot, rotationSpring, dt);
            _flip.Step(_facing, flipSpring, dt);
            _bob.Step(bob01 * bobHeight * speed01, bobSpring, dt);

            // ---------------- Uygula
            float facingSign = spriteFacesLeft ? -_flip.Value : _flip.Value;
            transform.localScale = new Vector3(
                _baseScale.x * Mathf.Max(0.05f, _scaleX.Value) * facingSign,
                _baseScale.y * Mathf.Max(0.05f, _scaleY.Value),
                _baseScale.z);
            transform.localPosition = _basePosition + new Vector3(0f, Mathf.Max(0f, _bob.Value), 0f);
            transform.localRotation = Quaternion.Euler(0f, 0f, _rotation.Value);
        }

        // ================================================================== Dış API (hasar, toplama, level-up...)

        /// <summary>Ölçeğe darbe. (0.3, -0.3) = yatay uzama + dikey basılma. Değerler yaklaşık sapma miktarıdır.</summary>
        public void PunchScale(Vector2 displacement)
        {
            _scaleX.AddDisplacementImpulse(displacement.x, scaleSpring);
            _scaleY.AddDisplacementImpulse(displacement.y, scaleSpring);
        }

        /// <summary>Hasar alınca: PunchRotation(Random.Range(-12, 12)).</summary>
        public void PunchRotation(float degrees)
        {
            _rotation.AddDisplacementImpulse(degrees, rotationSpring);
        }

        /// <summary>Pozitif = ezilme (geniş-basık), negatif = uzama (dar-uzun).</summary>
        public void Squash(float amount) => PunchScale(new Vector2(amount, -amount));

        /// <summary>Anlık yön kilitleme (ör. saldırı yönüne bakması için).</summary>
        public void FaceDirection(float xSign)
        {
            if (Mathf.Abs(xSign) > 0.001f) _facing = Mathf.Sign(xSign);
        }

        // ================================================================== Dash tepkileri

        private void OnDashStarted(Vector2 dir)
        {
            float ax = Mathf.Abs(dir.x);
            float ay = Mathf.Abs(dir.y);

            // Yatay dash -> yatay uzama; dikey dash -> dikey uzama
            PunchScale(new Vector2((ax - ay) * dashStretch, (ay - ax) * dashStretch));

            if (Mathf.Abs(dir.x) > 0.1f)
            {
                _facing = Mathf.Sign(dir.x);
                PunchRotation(-Mathf.Sign(dir.x) * dashTilt);
            }
        }

        private void OnDashEnded()
        {
            Squash(dashEndSquash); // "fren" ezilmesi
        }
    }
}
