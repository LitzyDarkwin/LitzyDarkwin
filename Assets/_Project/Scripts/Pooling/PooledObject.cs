using UnityEngine;

namespace Roguelite.Pooling
{
    /// <summary>
    /// Havuzdan gelen her instance'a OTOMATİK eklenir (prefab'a elle eklemene gerek yok, istersen ekleyebilirsin).
    /// Nesnenin hangi havuza ait olduğunu bilir -> Despawn() çağrısı O(1).
    /// Ayrıca coroutine'siz (GC'siz) "X saniye sonra havuza dön" zamanlayıcısı sunar.
    /// </summary>
    [DisallowMultipleComponent]
    public sealed class PooledObject : MonoBehaviour
    {
        private PrefabPool _owner;
        private IPoolable[] _poolables;
        private Component _cachedComponent;
        private float _despawnTimer;

        public bool IsSpawned { get; private set; }
        public bool HasOwner => _owner != null;

        internal void Bind(PrefabPool owner)
        {
            _owner = owner;
            _poolables = GetComponentsInChildren<IPoolable>(true);
            enabled = false; // Update sadece zamanlayıcı aktifken çalışsın
        }

        /// <summary>Spawn&lt;T&gt; için tip önbelleği. Aynı prefab hep aynı T ile çağrıldığından pratikte GetComponent bir kez çalışır.</summary>
        internal T GetCachedComponent<T>() where T : Component
        {
            if (_cachedComponent is T cached) return cached;
            T found = GetComponent<T>();
            _cachedComponent = found;
            return found;
        }

        internal void NotifySpawned()
        {
            IsSpawned = true;
            for (int i = 0; i < _poolables.Length; i++)
                _poolables[i].OnSpawned();
        }

        internal void NotifyDespawned()
        {
            IsSpawned = false;
            enabled = false;
            for (int i = 0; i < _poolables.Length; i++)
                _poolables[i].OnDespawned();
        }

        /// <summary>Nesneyi havuzuna geri gönderir. Havuzsuz nesneyse Destroy eder.</summary>
        public void Despawn()
        {
            if (_owner != null) _owner.Release(this);
            else Destroy(gameObject);
        }

        /// <summary>Partiküller, hasar yazıları, ghost'lar için: belirtilen süre sonra otomatik despawn.</summary>
        public void DespawnAfter(float seconds)
        {
            _despawnTimer = seconds;
            enabled = true;
        }

        public void CancelDespawnTimer() => enabled = false;

        private void Update()
        {
            _despawnTimer -= Time.deltaTime;
            if (_despawnTimer <= 0f)
            {
                enabled = false;
                Despawn();
            }
        }
    }
}
