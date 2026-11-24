using UnityEngine;

namespace Roguelite.CameraSystem
{
    /// <summary>
    /// GameObject: Main Camera. Hedefi (Player) yumuşak takip eder.
    /// (İleride Cinemachine'e geçilebilir; ADIM 3'teki Camera Shake bu kameranın çocuğuna eklenecek.)
    /// </summary>
    public sealed class SmoothCameraFollow : MonoBehaviour
    {
        [SerializeField] private Transform target;
        [SerializeField, Min(0f)] private float smoothTime = 0.12f;
        [SerializeField] private Vector2 offset = Vector2.zero;

        private Vector3 _velocity;

        public void SetTarget(Transform t) => target = t;

        private void LateUpdate()
        {
            if (target == null) return;
            Vector3 desired = new Vector3(target.position.x + offset.x, target.position.y + offset.y, transform.position.z);
            transform.position = Vector3.SmoothDamp(transform.position, desired, ref _velocity, smoothTime);
        }
    }
}
