using Roguelite.Core;
using Roguelite.Pooling;
using UnityEngine;

namespace Roguelite.Juice
{
    /// <summary>
    /// GameObject: Player KÖK objesine ekle (PlayerController ile aynı obje).
    ///
    /// Dash sırasında karakterin o anki sprite'ının (squash/stretch dahil!) donmuş kopyalarını bırakır
    /// ve bunlar renk gradyanıyla solar. Kopyalar PoolManager'dan gelir -> sıfır Instantiate.
    ///
    /// Kurulum:
    ///  - sourceRenderer: Visual/Sprite altındaki SpriteRenderer
    ///  - ghostPrefab   : GhostSprite prefab'ı (SpriteRenderer + GhostSprite scripti)
    ///  - optionalTrail : İstersen Visual altına bir TrailRenderer koy (Emitting kapalı başlasın), dash'te açılır.
    /// </summary>
    [DefaultExecutionOrder(100)] // ProceduralJuiceAnimator ölçeği uyguladıktan sonra kopyalansın
    public sealed class DashGhostTrail : MonoBehaviour
    {
        [SerializeField] private SpriteRenderer sourceRenderer;
        [SerializeField] private GhostSprite ghostPrefab;
        [SerializeField] private TrailRenderer optionalTrail;

        [Header("Ayarlar")]
        [SerializeField, Min(0.005f)] private float spawnInterval = 0.025f;
        [SerializeField, Min(0.05f)] private float ghostLifetime = 0.28f;
        [Tooltip("Başta parlak cyan, sonda saydam mor gibi bir gradyan önerilir.")]
        [SerializeField] private Gradient colorOverLifetime;
        [SerializeField, Range(0.5f, 1.2f)] private float endScale = 0.85f;
        [SerializeField, Min(0)] private int prewarmCount = 24;

        private IDashEventSource _dashSource;
        private bool _active;
        private float _timer;

        private void Awake()
        {
            _dashSource = GetComponent<IDashEventSource>();
            if (colorOverLifetime == null || colorOverLifetime.colorKeys.Length == 0)
                colorOverLifetime = DefaultGradient();
            if (ghostPrefab != null)
                PoolManager.Instance.Prewarm(ghostPrefab.gameObject, prewarmCount, 64);
            if (optionalTrail != null) optionalTrail.emitting = false;
        }

        private void OnEnable()
        {
            if (_dashSource == null) return;
            _dashSource.DashStarted += OnDashStarted;
            _dashSource.DashEnded += OnDashEnded;
        }

        private void OnDisable()
        {
            if (_dashSource == null) return;
            _dashSource.DashStarted -= OnDashStarted;
            _dashSource.DashEnded -= OnDashEnded;
            _active = false;
        }

        private void OnDashStarted(Vector2 direction)
        {
            _active = true;
            _timer = 0f;
            SpawnGhost();
            if (optionalTrail != null)
            {
                optionalTrail.Clear();
                optionalTrail.emitting = true;
            }
        }

        private void OnDashEnded()
        {
            _active = false;
            SpawnGhost();
            if (optionalTrail != null) optionalTrail.emitting = false;
        }

        private void LateUpdate() // LateUpdate: juice animatörü ölçeği uyguladıktan SONRA kopyala
        {
            if (!_active) return;

            _timer += Time.deltaTime;
            while (_timer >= spawnInterval)
            {
                _timer -= spawnInterval;
                SpawnGhost();
            }
        }

        private void SpawnGhost()
        {
            if (ghostPrefab == null || sourceRenderer == null || sourceRenderer.sprite == null) return;

            Transform src = sourceRenderer.transform;
            GhostSprite ghost = PoolManager.Instance.Spawn(ghostPrefab, src.position, src.rotation);
            ghost.Play(sourceRenderer, colorOverLifetime, ghostLifetime, endScale);
        }

#if UNITY_EDITOR
        private void Reset() => colorOverLifetime = DefaultGradient();
#endif

        public static Gradient DefaultGradient()
        {
            var g = new Gradient();
            g.SetKeys(
                new[]
                {
                    new GradientColorKey(new Color(0.55f, 0.95f, 1f), 0f),
                    new GradientColorKey(new Color(0.6f, 0.35f, 1f), 1f)
                },
                new[]
                {
                    new GradientAlphaKey(0.75f, 0f),
                    new GradientAlphaKey(0f, 1f)
                });
            return g;
        }
    }
}
