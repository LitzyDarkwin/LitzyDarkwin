using System;
using UnityEngine;

namespace Roguelite.Controls
{
    /// <summary>
    /// UI (joystick/buton) ile Oyuncu arasındaki köprü. İkisi birbirini TANIMAZ; ikisi de bu asset'e referans verir.
    /// Avantaj: Player bir prefab olarak sonradan spawn edilse bile sahnedeki UI'a referans gerekmez,
    /// ileride gamepad / AI / replay girdisi eklemek = sadece bu kanala yazan yeni bir bileşen.
    ///
    /// Oluştur: Create > Roguelite > Input > Input Channel  (projede TEK asset yeterli: "InputChannel")
    /// </summary>
    [CreateAssetMenu(fileName = "InputChannel", menuName = "Roguelite/Input/Input Channel")]
    public sealed class InputChannelSO : ScriptableObject
    {
        [NonSerialized] private Vector2 _touchMove;
        [NonSerialized] private Vector2 _keyboardMove;
        [NonSerialized] private bool _inputEnabled = true;

        /// <summary>Dash isteği (buton basıldığı AN tetiklenir, bırakınca değil -> daha hızlı his).</summary>
        public event Action DashRequested;

        /// <summary>Son hareket yönü (0..1 büyüklük). Dokunma girdisi klavyeye göre önceliklidir.</summary>
        public Vector2 Move
        {
            get
            {
                if (!_inputEnabled) return Vector2.zero;
                return _touchMove.sqrMagnitude >= _keyboardMove.sqrMagnitude ? _touchMove : _keyboardMove;
            }
        }

        public bool InputEnabled
        {
            get => _inputEnabled;
            set => _inputEnabled = value; // Level-up ekranı açıkken false yap
        }

        public void SetTouchMove(Vector2 value) => _touchMove = Vector2.ClampMagnitude(value, 1f);

        public void SetKeyboardMove(Vector2 value) => _keyboardMove = Vector2.ClampMagnitude(value, 1f);

        public void RequestDash()
        {
            if (_inputEnabled) DashRequested?.Invoke();
        }

        private void OnEnable()
        {
            _touchMove = Vector2.zero;
            _keyboardMove = Vector2.zero;
            _inputEnabled = true;
        }
    }
}
