using UnityEngine;

namespace Roguelite.Pooling
{
    /// <summary>
    /// Tek bir prefab için Unity-farkında havuz. Generic ObjectPool&lt;PooledObject&gt; üzerine kuruludur.
    /// Doğrudan kullanmak yerine genelde PoolManager üzerinden erişilir.
    /// </summary>
    public sealed class PrefabPool
    {
        private readonly GameObject _prefab;
        private readonly Transform _container;
        private readonly ObjectPool<PooledObject> _pool;

        public GameObject Prefab => _prefab;
        public int CountActive => _pool.CountActive;
        public int CountInactive => _pool.CountInactive;

        /// <param name="container">
        /// Pasif nesnelerin bekletileceği parent. PERFORMANS NOTU: Yüzlerce hareketli nesne tek bir parent altında olursa
        /// Unity transform güncellemelerini paralelleştiremez. Bu yüzden spawn edilen nesneler sahne köküne (parent=null)
        /// alınır, sadece pasifken container altında durur.
        /// </param>
        public PrefabPool(GameObject prefab, Transform container, int maxSize)
        {
            _prefab = prefab;
            _container = container;
            _pool = new ObjectPool<PooledObject>(
                create: CreateInstance,
                onGet: null,
                onRelease: OnRelease,
                onDestroy: po => Object.Destroy(po.gameObject),
                isValid: po => po != null, // Unity null-check (yok edilmiş mi?)
                initialCapacity: 64,
                maxSize: maxSize);
        }

        private PooledObject CreateInstance()
        {
            GameObject go = Object.Instantiate(_prefab, _container);
            go.name = _prefab.name; // "(Clone)" eki olmasın
            if (!go.TryGetComponent(out PooledObject po))
                po = go.AddComponent<PooledObject>();
            po.Bind(this);
            go.SetActive(false);
            return po;
        }

        public PooledObject Spawn(Vector3 position, Quaternion rotation, Transform parent = null)
        {
            PooledObject po = _pool.Get();
            Transform tr = po.transform;

            if (tr.parent != parent)
                tr.SetParent(parent, false);
            tr.SetPositionAndRotation(position, rotation);

            po.gameObject.SetActive(true); // önce pozisyon, sonra aktivasyon: OnEnable doğru konumu görür
            po.NotifySpawned();
            return po;
        }

        internal void Release(PooledObject po)
        {
            if (!po.IsSpawned)
            {
#if UNITY_EDITOR
                Debug.LogWarning($"[Pool] '{po.name}' zaten havuzda. Çift Despawn çağrısı engellendi.", po);
#endif
                return;
            }
            _pool.Release(po);
        }

        private void OnRelease(PooledObject po)
        {
            po.NotifyDespawned();
            po.gameObject.SetActive(false);
            Transform tr = po.transform;
            if (tr.parent != _container)
                tr.SetParent(_container, false);
        }

        public void Prewarm(int count) => _pool.Prewarm(count);

        public void Clear() => _pool.Clear();
    }
}
