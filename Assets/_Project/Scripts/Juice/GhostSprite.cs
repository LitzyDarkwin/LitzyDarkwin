using Roguelite.Pooling;
using UnityEngine;

namespace Roguelite.Juice
{
    /// <summary>
    /// PREFAB: "GhostSprite" adlı boş bir prefab oluştur:
    ///   GhostSprite (SpriteRenderer + bu script)
    ///   - SpriteRenderer Material: düz renk silüet için "Sprites/Default" yeterli; daha parlak (additive) görünüm için
    ///     Blend = Additive olan bir URP 2D Sprite materyali kullanabilirsin.
    /// DashGhostTrail bu prefab'ı PoolManager üzerinden çoğaltır.
    /// </summary>
    [RequireComponent(typeof(SpriteRenderer))]
    public sealed class GhostSprite : MonoBehaviour, IPoolable
    {
        private SpriteRenderer _renderer;
        private PooledObject _pooled;
        private Gradient _gradient;
        private Vector3 _startScale;
        private float _endScale;
        private float _lifetime;
        private float _age;

        private void Awake()
        {
            _renderer = GetComponent<SpriteRenderer>();
            _pooled = GetComponent<PooledObject>();
        }

        public void Play(SpriteRenderer source, Gradient gradient, float lifetime, float endScale)
        {
            if (_pooled == null) _pooled = GetComponent<PooledObject>();

            _renderer.sprite = source.sprite;
            _renderer.flipX = source.flipX;
            _renderer.flipY = source.flipY;
            _renderer.sortingLayerID = source.sortingLayerID;
            _renderer.sortingOrder = source.sortingOrder - 1; // karakterin arkasında

            // Kaynak dünya ölçeğini (squash, yön çevirme dahil) dondur
            transform.localScale = source.transform.lossyScale;
            _startScale = transform.localScale;

            _gradient = gradient;
            _lifetime = Mathf.Max(0.01f, lifetime);
            _endScale = endScale;
            _age = 0f;
            _renderer.color = gradient.Evaluate(0f);
        }

        private void Update()
        {
            _age += Time.deltaTime;
            float t = _age / _lifetime;

            if (t >= 1f)
            {
                if (_pooled != null) _pooled.Despawn();
                else Destroy(gameObject);
                return;
            }

            _renderer.color = _gradient.Evaluate(t);
            transform.localScale = _startScale * Mathf.Lerp(1f, _endScale, t);
        }

        public void OnSpawned() { }

        public void OnDespawned()
        {
            _renderer.sprite = null;
            _gradient = null;
        }
    }
}
