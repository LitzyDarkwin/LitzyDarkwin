using UnityEngine;
#if ENABLE_INPUT_SYSTEM
using UnityEngine.InputSystem;
#endif

namespace Roguelite.Controls
{
    /// <summary>
    /// GameObject: "[Systems]" objesine ekle. Editörde/PC'de test için WASD/Ok tuşları + Space/Shift = Dash.
    /// Hem yeni Input System hem eski Input Manager ile derlenir. Mobil build'de kendini kapatabilir.
    /// </summary>
    public sealed class KeyboardInputBridge : MonoBehaviour
    {
        [SerializeField] private InputChannelSO inputChannel;
        [SerializeField] private bool disableOnMobileBuilds = true;

        private void Awake()
        {
            if (disableOnMobileBuilds && Application.isMobilePlatform && !Application.isEditor)
                enabled = false;
        }

        private void OnDisable()
        {
            if (inputChannel != null) inputChannel.SetKeyboardMove(Vector2.zero);
        }

        private void Update()
        {
            Vector2 move = Vector2.zero;
            bool dash = false;

#if ENABLE_INPUT_SYSTEM
            Keyboard kb = Keyboard.current;
            if (kb != null)
            {
                if (kb.aKey.isPressed || kb.leftArrowKey.isPressed) move.x -= 1f;
                if (kb.dKey.isPressed || kb.rightArrowKey.isPressed) move.x += 1f;
                if (kb.sKey.isPressed || kb.downArrowKey.isPressed) move.y -= 1f;
                if (kb.wKey.isPressed || kb.upArrowKey.isPressed) move.y += 1f;
                dash = kb.spaceKey.wasPressedThisFrame || kb.leftShiftKey.wasPressedThisFrame;
            }
#elif ENABLE_LEGACY_INPUT_MANAGER
            move = new Vector2(Input.GetAxisRaw("Horizontal"), Input.GetAxisRaw("Vertical"));
            dash = Input.GetKeyDown(KeyCode.Space) || Input.GetKeyDown(KeyCode.LeftShift);
#endif

            inputChannel.SetKeyboardMove(move.sqrMagnitude > 1f ? move.normalized : move);
            if (dash) inputChannel.RequestDash();
        }
    }
}
