using System;
using System.Collections.Generic;
using UnityEngine;

namespace Roguelite.Pooling
{
    /// <summary>
    /// GameObject: Sahnedeki "[Systems]" objesine ekle (yoksa ilk erişimde kendini otomatik oluşturur).
    ///
    /// Tek giriş noktası:
    ///   Enemy e   = PoolManager.Instance.Spawn(enemyPrefab, pos, Quaternion.identity);
    ///   Bullet b  = PoolManager.Instance.Spawn(bulletPrefab, pos, rot);
    ///   PoolManager.Despawn(e);        // veya e.GetComponent&lt;PooledObject&gt;().Despawn();
    ///
    /// Havuzlar prefab GameObject'inin InstanceID'si ile anahtarlanır; ilk Spawn'da havuz otomatik açılır.
    /// "Presets" listesine eklenen prefab'lar Awake'te önceden üretilir (oyun içi Instantiate spike'ı = 0).
    /// </summary>
    [DefaultExecutionOrder(-1000)]
    [DisallowMultipleComponent]
    public sealed class PoolManager : MonoBehaviour
    {
        [Serializable]
        public class PoolPreset
        {
            public GameObject Prefab;
            [Min(0)] public int Prewarm = 32;
            [Min(1)] public int MaxSize = 512;
        }

        [Tooltip("Sahne açılırken önceden üretilecek prefab'lar (düşmanlar, mermiler, hasar yazıları, XP taşları...).")]
        [SerializeField] private List<PoolPreset> presets = new List<PoolPreset>();

        [Tooltip("Preset'te olmayan, ilk kez spawn edilen prefab'lar için varsayılan maksimum pasif nesne sayısı.")]
        [SerializeField, Min(1)] private int defaultMaxSize = 256;

        private static PoolManager _instance;
        private readonly Dictionary<int, PrefabPool> _pools = new Dictionary<int, PrefabPool>(64);
        private Transform _root;

        public static PoolManager Instance
        {
            get
            {
                if (_instance == null)
                {
                    _instance = FindAnyObjectByType<PoolManager>();
                    if (_instance == null)
                        _instance = new GameObject("[PoolManager]").AddComponent<PoolManager>();
                }
                return _instance;
            }
        }

        private void Awake()
        {
            if (_instance != null && _instance != this)
            {
                Destroy(gameObject);
                return;
            }
            _instance = this;

            _root = new GameObject("PooledObjects").transform;
            _root.SetParent(transform, false);

            for (int i = 0; i < presets.Count; i++)
            {
                PoolPreset p = presets[i];
                if (p.Prefab == null) continue;
                GetOrCreatePool(p.Prefab, p.MaxSize).Prewarm(p.Prewarm);
            }
        }

        private void OnDestroy()
        {
            if (_instance == this) _instance = null;
        }

        // ------------------------------------------------------------------ API

        /// <summary>Generic spawn: prefab'ı hangi bileşen tipiyle verirsen o tipte instance döner.</summary>
        public T Spawn<T>(T prefab, Vector3 position, Quaternion rotation, Transform parent = null) where T : Component
        {
            PooledObject po = GetOrCreatePool(prefab.gameObject, defaultMaxSize).Spawn(position, rotation, parent);
            return po.GetCachedComponent<T>();
        }

        public GameObject Spawn(GameObject prefab, Vector3 position, Quaternion rotation, Transform parent = null)
        {
            return GetOrCreatePool(prefab, defaultMaxSize).Spawn(position, rotation, parent).gameObject;
        }

        public void Prewarm(GameObject prefab, int count, int maxSize = -1)
        {
            GetOrCreatePool(prefab, maxSize > 0 ? maxSize : defaultMaxSize).Prewarm(count);
        }

        public static void Despawn(Component instance)
        {
            if (instance != null) Despawn(instance.gameObject);
        }

        public static void Despawn(GameObject instance)
        {
            if (instance == null) return;
            if (instance.TryGetComponent(out PooledObject po)) po.Despawn();
            else Destroy(instance);
        }

        public static void DespawnAfter(GameObject instance, float seconds)
        {
            if (instance != null && instance.TryGetComponent(out PooledObject po)) po.DespawnAfter(seconds);
            else if (instance != null) Destroy(instance, seconds);
        }

        /// <summary>Debug / profil için: prefab başına aktif nesne sayısı.</summary>
        public int GetActiveCount(GameObject prefab)
        {
            return _pools.TryGetValue(prefab.GetInstanceID(), out PrefabPool pool) ? pool.CountActive : 0;
        }

        // ------------------------------------------------------------------ İç

        private PrefabPool GetOrCreatePool(GameObject prefab, int maxSize)
        {
            int key = prefab.GetInstanceID();
            if (_pools.TryGetValue(key, out PrefabPool pool))
                return pool;

            Transform container = new GameObject($"Pool_{prefab.name}").transform;
            container.SetParent(_root, false);

            pool = new PrefabPool(prefab, container, maxSize);
            _pools.Add(key, pool);
            return pool;
        }
    }
}
