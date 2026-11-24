using UnityEngine;

namespace Roguelite.Core.Utils
{
    /// <summary>Unity 6 (linearVelocity) ve 2022 LTS (velocity) arasında uyumluluk katmanı.</summary>
    public static class Rigidbody2DExtensions
    {
        public static Vector2 GetVelocity(this Rigidbody2D rb)
        {
#if UNITY_6000_0_OR_NEWER
            return rb.linearVelocity;
#else
            return rb.velocity;
#endif
        }

        public static void SetVelocity(this Rigidbody2D rb, Vector2 velocity)
        {
#if UNITY_6000_0_OR_NEWER
            rb.linearVelocity = velocity;
#else
            rb.velocity = velocity;
#endif
        }
    }
}
