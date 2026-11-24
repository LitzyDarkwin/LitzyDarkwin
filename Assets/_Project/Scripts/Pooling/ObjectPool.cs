using System;
using System.Collections.Generic;

namespace Roguelite.Pooling
{
    /// <summary>
    /// Saf C# GENERIC havuz çekirdeği. Unity'den bağımsızdır; hem prefab havuzu (PrefabPool) hem de
    /// List&lt;T&gt;, DamageInfo, hedef listeleri gibi saf C# nesneleri için kullanılabilir.
    ///
    ///   var pool = new ObjectPool&lt;List&lt;Enemy&gt;&gt;(() => new List&lt;Enemy&gt;(64), onRelease: l => l.Clear());
    ///   var list = pool.Get(); ... pool.Release(list);
    /// </summary>
    public sealed class ObjectPool<T> where T : class
    {
        private readonly Stack<T> _inactive;
        private readonly Func<T> _create;
        private readonly Action<T> _onGet;
        private readonly Action<T> _onRelease;
        private readonly Action<T> _onDestroy;
        private readonly Func<T, bool> _isValid;
        private readonly int _maxSize;

        public int CountAll { get; private set; }
        public int CountInactive => _inactive.Count;
        public int CountActive => CountAll - CountInactive;

        /// <param name="create">Yeni nesne üretir (havuz boşsa).</param>
        /// <param name="onGet">Havuzdan alınırken.</param>
        /// <param name="onRelease">Havuza dönerken.</param>
        /// <param name="onDestroy">Havuz kapasitesi dolduysa / Clear'da nesne atılırken.</param>
        /// <param name="isValid">Unity nesneleri dışarıdan yok edilmiş olabilir (sahne değişimi) -> false dönerse atlanır.</param>
        /// <param name="maxSize">Havuzda bekletilecek en fazla pasif nesne. Fazlası onDestroy'a gider.</param>
        public ObjectPool(Func<T> create,
                          Action<T> onGet = null,
                          Action<T> onRelease = null,
                          Action<T> onDestroy = null,
                          Func<T, bool> isValid = null,
                          int initialCapacity = 32,
                          int maxSize = 1024)
        {
            _create = create ?? throw new ArgumentNullException(nameof(create));
            _onGet = onGet;
            _onRelease = onRelease;
            _onDestroy = onDestroy;
            _isValid = isValid;
            _maxSize = Math.Max(1, maxSize);
            _inactive = new Stack<T>(initialCapacity);
        }

        public T Get()
        {
            T item = null;
            while (_inactive.Count > 0)
            {
                T candidate = _inactive.Pop();
                if (_isValid == null || _isValid(candidate))
                {
                    item = candidate;
                    break;
                }
                CountAll--; // dışarıdan yok edilmiş, sayaçtan düş
            }

            if (item == null)
            {
                item = _create();
                CountAll++;
            }

            _onGet?.Invoke(item);
            return item;
        }

        public void Release(T item)
        {
            if (item == null) return;

            _onRelease?.Invoke(item);

            if (_inactive.Count < _maxSize)
            {
                _inactive.Push(item);
            }
            else
            {
                _onDestroy?.Invoke(item);
                CountAll--;
            }
        }

        /// <summary>Yükleme ekranında çağır: oyun sırasında Instantiate spike'ı olmasın.</summary>
        public void Prewarm(int count)
        {
            int toCreate = Math.Min(count, _maxSize) - _inactive.Count;
            for (int i = 0; i < toCreate; i++)
            {
                T item = _create();
                CountAll++;
                _inactive.Push(item);
            }
        }

        public void Clear()
        {
            while (_inactive.Count > 0)
            {
                T item = _inactive.Pop();
                if (_isValid == null || _isValid(item))
                    _onDestroy?.Invoke(item);
            }
            CountAll = 0;
        }
    }
}
