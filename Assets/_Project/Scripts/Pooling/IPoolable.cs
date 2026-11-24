namespace Roguelite.Pooling
{
    /// <summary>
    /// Havuzlanan prefab'ın herhangi bir bileşeni (kökte veya çocukta) bunu uygulayabilir.
    /// Sıra:  SetActive(true) -> OnEnable -> OnSpawned()   |   OnDespawned() -> SetActive(false) -> OnDisable
    /// Start() yalnızca İLK aktivasyonda çalışır; her spawn'da sıfırlanması gereken durumu OnSpawned'a koy.
    /// </summary>
    public interface IPoolable
    {
        void OnSpawned();
        void OnDespawned();
    }
}
