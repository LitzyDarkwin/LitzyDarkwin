using System;
using Roguelite.Controls;
using Roguelite.Core;
using Roguelite.Core.Events;
using Roguelite.Core.Utils;
using Roguelite.Stats;
using UnityEngine;

namespace Roguelite.Player
{
    /// <summary>
    /// GameObject: Player KÖK objesine ekle.
    /// Gerekli: Rigidbody2D (otomatik eklenir: Dynamic, Gravity 0, Freeze Rotation Z, Interpolate),
    ///          CircleCollider2D (radius ~0.35), CharacterStats.
    ///
    /// Hareket: Hedef hıza üstel (exponential) yumuşatma ile yaklaşır -> kare hızından bağımsız "yağ gibi" his.
    ///          Ani yön değişimlerinde ayrı "turn" keskinliği ile tepki gecikmesi hissedilmez.
    /// Dash:    Eğri (AnimationCurve) tabanlı mesafe, çoklu şarj, input buffer, i-frame, düşmanların içinden geçme,
    ///          dash sonu momentum taşıma. Duvarlara karşı fizik korunur (velocity tabanlı, teleport değil).
    ///
    /// Haberleşme:
    ///   - Aynı objedeki görsel bileşenlere: C# event (DashStarted / DashEnded)   [IDashEventSource]
    ///   - Global sistemlere (UI, kamera, ses, rünler): EventBus
    /// </summary>
    [RequireComponent(typeof(Rigidbody2D))]
    [RequireComponent(typeof(CharacterStats))]
    [DisallowMultipleComponent]
    public sealed class PlayerController : MonoBehaviour, IVelocityProvider, IDashEventSource, IInvulnerable
    {
        [Header("Girdi")]
        [SerializeField] private InputChannelSO inputChannel;

        [Header("Hareket Hissi")]
        [Tooltip("Hızlanma keskinliği. Yüksek = daha 'snappy'. 15-25 arası mobilde iyi hissettirir.")]
        [SerializeField] private float accelerationSharpness = 20f;
        [Tooltip("Parmak kalkınca durma keskinliği. Biraz düşük = hafif kayma (tatlı), çok düşük = buz pisti.")]
        [SerializeField] private float decelerationSharpness = 14f;
        [Tooltip("Yön >90° değişirken kullanılan keskinlik (U dönüşlerinde yavaşlama hissini engeller).")]
        [SerializeField] private float turnSharpness = 32f;

        [Header("Dash")]
        [SerializeField, Min(0.03f)] private float dashDuration = 0.16f;
        [Tooltip("X: normalize zaman, Y: normalize mesafe. Hızlı başlayıp yumuşak biten (ease-out) eğri önerilir.")]
        [SerializeField] private AnimationCurve dashCurve = new AnimationCurve(
            new Keyframe(0f, 0f, 0f, 2.6f),
            new Keyframe(1f, 1f, 0.15f, 0f));
        [Tooltip("Dash bittikten sonra ek dokunulmazlık süresi (kalabalıktan çıkarken affedicilik).")]
        [SerializeField, Min(0f)] private float extraInvulnerability = 0.1f;
        [Tooltip("Dash bekleme süresindeyken basılırsa, bu süre içinde hazır olunca otomatik atılır.")]
        [SerializeField, Min(0f)] private float dashInputBuffer = 0.15f;
        [Tooltip("Dash sonrası hareket hızının yüzde kaçı korunur (akıcı geçiş).")]
        [SerializeField, Range(0f, 2f)] private float dashExitSpeedFactor = 1f;

        [Header("Fizik Katmanları")]
        [Tooltip("Dash/i-frame sırasında içinden geçilecek katmanlar (ör. 'Enemy', 'EnemyProjectile').")]
        [SerializeField] private LayerMask passThroughLayers;

        // ---- Bileşenler
        private Rigidbody2D _rb;
        private CharacterStats _stats;

        // ---- Hareket durumu
        private Vector2 _velocity;
        private Vector2 _facing = Vector2.right;

        // ---- Dash durumu
        private bool _isDashing;
        private float _dashElapsed;
        private float _dashPrevCurve;
        private float _dashDistance;
        private Vector2 _dashDirection;
        private float _dashBufferTimer;
        private float _invulnerableTimer;
        private bool _collisionsIgnored;

        private int _charges;
        private float _rechargeTimer;
        private float _rechargeDuration;

        // ---- Arayüzler / public API
        public Vector2 Velocity => _velocity;
        public float MaxSpeed => _stats[StatType.MoveSpeed];
        public Vector2 FacingDirection => _facing;
        public Vector2 MoveInput => inputChannel != null ? inputChannel.Move : Vector2.zero;
        public bool IsDashing => _isDashing;
        public bool IsInvulnerable => _invulnerableTimer > 0f || _isDashing;
        public int DashCharges => _charges;
        public int MaxDashCharges => Mathf.Max(1, _stats.GetInt(StatType.DashCharges));

        public event Action<Vector2> DashStarted;
        public event Action DashEnded;

        // ================================================================== Unity

        private void Awake()
        {
            _rb = GetComponent<Rigidbody2D>();
            _stats = GetComponent<CharacterStats>();

            _rb.gravityScale = 0f;
            _rb.freezeRotation = true;
            _rb.interpolation = RigidbodyInterpolation2D.Interpolate;
            _rb.collisionDetectionMode = CollisionDetectionMode2D.Continuous; // hızlı dash'te duvardan kaçmasın
        }

        private void OnEnable()
        {
            if (inputChannel != null) inputChannel.DashRequested += OnDashRequested;

            _charges = MaxDashCharges;
            _rechargeTimer = 0f;
            BroadcastCharges();
        }

        private void OnDisable()
        {
            if (inputChannel != null) inputChannel.DashRequested -= OnDashRequested;
            SetEnemyCollisionIgnored(false);
            _isDashing = false;
        }

        private void Update()
        {
            float dt = Time.deltaTime;

            Vector2 move = MoveInput;
            if (move.sqrMagnitude > 0.01f && !_isDashing)
                _facing = move.normalized;

            TickRecharge(dt);

            // Dokunulmazlık bitince çarpışmaları geri aç
            if (_invulnerableTimer > 0f)
            {
                _invulnerableTimer -= dt;
                if (_invulnerableTimer <= 0f && !_isDashing)
                    SetEnemyCollisionIgnored(false);
            }

            // Input buffer: bekleme sırasında basılan dash'i kaybetme
            if (_dashBufferTimer > 0f)
            {
                _dashBufferTimer -= dt;
                if (CanDash()) StartDash();
            }
        }

        private void FixedUpdate()
        {
            float dt = Time.fixedDeltaTime;
            if (_isDashing) DashStep(dt);
            else MoveStep(dt);
        }

        // ================================================================== Hareket

        private void MoveStep(float dt)
        {
            Vector2 target = MoveInput * MaxSpeed;

            float sharpness;
            if (target.sqrMagnitude < 0.0001f)
                sharpness = decelerationSharpness;
            else if (Vector2.Dot(target, _velocity) < 0f)
                sharpness = turnSharpness;
            else
                sharpness = accelerationSharpness;

            // Kare hızından bağımsız üstel yumuşatma: lerp(a, b, 1 - e^(-k*dt))
            _velocity = Vector2.Lerp(_velocity, target, 1f - Mathf.Exp(-sharpness * dt));
            if (_velocity.sqrMagnitude < 0.0001f) _velocity = Vector2.zero;

            _rb.SetVelocity(_velocity);
        }

        // ================================================================== Dash

        private void OnDashRequested()
        {
            if (CanDash()) StartDash();
            else _dashBufferTimer = dashInputBuffer;
        }

        private bool CanDash() => !_isDashing && _charges > 0;

        private void StartDash()
        {
            _dashBufferTimer = 0f;

            // Yön: aktif girdi varsa o, yoksa son bakılan yön (duruyorken de dash atılabilsin)
            Vector2 move = MoveInput;
            _dashDirection = move.sqrMagnitude > 0.04f ? move.normalized : _facing;
            _facing = _dashDirection;

            _dashDistance = _stats[StatType.DashDistance];
            _dashElapsed = 0f;
            _dashPrevCurve = 0f;
            _isDashing = true;

            ConsumeCharge();
            SetEnemyCollisionIgnored(true);

            DashStarted?.Invoke(_dashDirection);
            EventBus<PlayerDashStartedEvent>.Raise(new PlayerDashStartedEvent
            {
                Direction = _dashDirection,
                Position = transform.position
            });
        }

        private void DashStep(float dt)
        {
            _dashElapsed += dt;
            float t = Mathf.Clamp01(_dashElapsed / dashDuration);
            float curve = dashCurve.Evaluate(t);

            // Eğrinin bu adımdaki artışı kadar yol al. Velocity ile hareket -> duvarlar hâlâ engeller.
            float stepDistance = (curve - _dashPrevCurve) * _dashDistance;
            _dashPrevCurve = curve;

            _velocity = _dashDirection * (stepDistance / dt);
            _rb.SetVelocity(_velocity);

            if (t >= 1f) EndDash();
        }

        private void EndDash()
        {
            _isDashing = false;
            _invulnerableTimer = extraInvulnerability;
            if (_invulnerableTimer <= 0f) SetEnemyCollisionIgnored(false);

            // Momentum taşı: girdi varsa o yöne, yoksa dash yönüne doğru hızla devam et ve yumuşakça yavaşla
            Vector2 move = MoveInput;
            Vector2 exitDir = move.sqrMagnitude > 0.04f ? move.normalized : _dashDirection;
            _velocity = exitDir * (MaxSpeed * dashExitSpeedFactor);
            _rb.SetVelocity(_velocity);

            DashEnded?.Invoke();
            EventBus<PlayerDashEndedEvent>.Raise(new PlayerDashEndedEvent { Position = transform.position });
        }

        /// <summary>Dash'i dışarıdan iptal et (ör. sersemletme, level-up ekranı).</summary>
        public void CancelDash()
        {
            if (_isDashing) EndDash();
        }

        // ================================================================== Şarj / Cooldown

        private void ConsumeCharge()
        {
            bool wasFull = _charges >= MaxDashCharges;
            _charges--;
            if (wasFull || _rechargeTimer <= 0f)
                BeginRecharge();
            BroadcastCharges();
        }

        private void BeginRecharge()
        {
            _rechargeDuration = StatFormulas.Cooldown(
                _stats[StatType.DashCooldown],
                _stats[StatType.CooldownReduction]);
            _rechargeTimer = _rechargeDuration;
        }

        private void TickRecharge(float dt)
        {
            int max = MaxDashCharges;

            // Eşya ile max şarj azaldıysa
            if (_charges > max)
            {
                _charges = max;
                BroadcastCharges();
            }

            if (_charges >= max) return;

            _rechargeTimer -= dt;
            if (_rechargeTimer > 0f) return;

            _charges++;
            if (_charges < max) BeginRecharge();
            else _rechargeTimer = 0f;
            BroadcastCharges();
        }

        private void BroadcastCharges()
        {
            EventBus<DashChargesChangedEvent>.Raise(new DashChargesChangedEvent
            {
                Current = _charges,
                Max = MaxDashCharges,
                RechargeRemaining = _charges < MaxDashCharges ? Mathf.Max(0f, _rechargeTimer) : 0f,
                RechargeDuration = _rechargeDuration
            });
        }

        // ================================================================== Dokunulmazlık

        /// <summary>Hasar alınca kısa i-frame vermek için (Adım 3'te Health kullanacak).</summary>
        public void GrantInvulnerability(float seconds, bool passThroughEnemies = false)
        {
            _invulnerableTimer = Mathf.Max(_invulnerableTimer, seconds);
            if (passThroughEnemies) SetEnemyCollisionIgnored(true);
        }

        private void SetEnemyCollisionIgnored(bool ignore)
        {
            if (_collisionsIgnored == ignore) return;
            _collisionsIgnored = ignore;

            int myLayer = gameObject.layer;
            int mask = passThroughLayers.value;
            for (int layer = 0; layer < 32; layer++)
            {
                if ((mask & (1 << layer)) != 0)
                    Physics2D.IgnoreLayerCollision(myLayer, layer, ignore);
            }
        }

#if UNITY_EDITOR
        private void OnDrawGizmosSelected()
        {
            Gizmos.color = Color.cyan;
            float dist = Application.isPlaying && _stats != null ? _stats[StatType.DashDistance] : 3.2f;
            Gizmos.DrawLine(transform.position, transform.position + (Vector3)(_facing * dist));
        }
#endif
    }
}
