using UnityEngine;
using UnityEngine.EventSystems;

namespace Roguelite.Controls
{
    /// <summary>
    /// GameObject: Canvas altındaki "JoystickArea" paneline ekle.
    ///
    /// Hiyerarşi:
    ///   Canvas (Screen Space - Overlay, CanvasScaler: Scale With Screen Size 1080x1920, Match 0.5)
    ///    └─ JoystickArea   (RectTransform: tüm ekranı kaplasın, Image alpha=0, Raycast Target = AÇIK)  &lt;-- BU SCRIPT
    ///        └─ Background (Image: halka sprite, ~220x220, Raycast Target = KAPALI)
    ///            └─ Handle (Image: dolu daire, ~100x100, Raycast Target = KAPALI)
    ///    └─ DashButton     (JoystickArea'dan SONRA gelsin ki üstte kalsın ve dokunuşu o yakalasın)
    ///
    /// Özellikler:
    ///  - Parmağın bastığı yerde belirir (floating), sürükleme yarıçapı aşarsa arkadan takip eder (dynamic follow)
    ///  - Multi-touch güvenli: ilk parmağı kilitler, ikinci parmak (dash butonu) karışmaz
    ///  - Dead zone + yanıt eğrisi (response exponent) -> mikro düzeltmeler hassas, tam itiş hızlı
    ///  - Belirme/kaybolma alpha animasyonu ve tutamağın yaylı geri dönüşü
    /// </summary>
    [RequireComponent(typeof(RectTransform))]
    public sealed class FloatingJoystick : MonoBehaviour, IPointerDownHandler, IDragHandler, IPointerUpHandler
    {
        private const int NoPointer = int.MinValue;

        [Header("Bağlantılar")]
        [SerializeField] private InputChannelSO inputChannel;
        [SerializeField] private RectTransform background;
        [SerializeField] private RectTransform handle;
        [Tooltip("Background üzerinde CanvasGroup (alpha animasyonu için). Boşsa otomatik eklenir.")]
        [SerializeField] private CanvasGroup visualGroup;

        [Header("Davranış")]
        [Tooltip("Tutamağın merkezden gidebileceği max mesafe (canvas birimi).")]
        [SerializeField, Min(10f)] private float radius = 110f;
        [SerializeField, Range(0f, 0.5f)] private float deadZone = 0.1f;
        [Tooltip("1 = doğrusal. 1.3-1.6 = merkeze yakın daha hassas kontrol.")]
        [SerializeField, Range(0.5f, 3f)] private float responseExponent = 1.25f;
        [Tooltip("Parmak yarıçapın dışına çıkınca joystick arkasından gelsin.")]
        [SerializeField] private bool dynamicFollow = true;
        [Tooltip("Sadece ekranın sol yarısında başlasın mı? (Sağ yarı ileride başka aksiyonlar için boş kalır)")]
        [SerializeField] private bool leftHalfOnly;

        [Header("Görsel")]
        [SerializeField, Range(0f, 1f)] private float idleAlpha = 0.25f;
        [SerializeField, Range(0f, 1f)] private float activeAlpha = 0.9f;
        [SerializeField] private float fadeSpeed = 12f;
        [SerializeField] private float handleReturnSpeed = 18f;
        [Tooltip("Bırakınca joystick varsayılan konumuna dönsün (idle göstergesi olarak).")]
        [SerializeField] private bool returnToDefaultOnRelease = true;
        [Tooltip("Basınca arka planın küçük 'pop' büyümesi.")]
        [SerializeField] private float pressPopScale = 1.12f;

        private RectTransform _area;
        private Canvas _canvas;
        private Vector2 _defaultBackgroundPos;
        private int _pointerId = NoPointer;
        private float _targetAlpha;
        private float _popScale = 1f;

        public Vector2 Output { get; private set; }
        public bool IsPressed => _pointerId != NoPointer;

        private void Awake()
        {
            _area = (RectTransform)transform;
            _canvas = GetComponentInParent<Canvas>().rootCanvas;

            // Background'un anchor'ını alanın pivot'una sabitle -> anchoredPosition == local nokta (matematik basitleşir)
            background.anchorMin = background.anchorMax = _area.pivot;
            handle.anchorMin = handle.anchorMax = new Vector2(0.5f, 0.5f);
            _defaultBackgroundPos = background.anchoredPosition;

            if (visualGroup == null)
            {
                visualGroup = background.GetComponent<CanvasGroup>();
                if (visualGroup == null) visualGroup = background.gameObject.AddComponent<CanvasGroup>();
            }
            visualGroup.blocksRaycasts = false;
            visualGroup.interactable = false;

            _targetAlpha = idleAlpha;
            visualGroup.alpha = idleAlpha;
        }

        private void OnDisable()
        {
            ResetStick();
        }

        public void OnPointerDown(PointerEventData eventData)
        {
            if (_pointerId != NoPointer) return; // zaten bir parmak kontrol ediyor
            if (leftHalfOnly && eventData.position.x > Screen.width * 0.5f) return;
            if (!ScreenToLocal(eventData.position, out Vector2 local)) return;

            _pointerId = eventData.pointerId;
            background.anchoredPosition = local;
            handle.anchoredPosition = Vector2.zero;
            _targetAlpha = activeAlpha;
            _popScale = pressPopScale;

            UpdateStick(local);
        }

        public void OnDrag(PointerEventData eventData)
        {
            if (eventData.pointerId != _pointerId) return;
            if (ScreenToLocal(eventData.position, out Vector2 local))
                UpdateStick(local);
        }

        public void OnPointerUp(PointerEventData eventData)
        {
            if (eventData.pointerId != _pointerId) return;
            ResetStick();
        }

        private void UpdateStick(Vector2 localPoint)
        {
            Vector2 offset = localPoint - background.anchoredPosition;
            float distance = offset.magnitude;

            if (distance > radius)
            {
                Vector2 dir = offset / distance;
                if (dynamicFollow)
                    background.anchoredPosition += dir * (distance - radius); // joystick parmağı takip eder
                offset = dir * radius;
                distance = radius;
            }

            handle.anchoredPosition = offset;

            float normalized = distance / radius;
            if (normalized < deadZone)
            {
                Output = Vector2.zero;
            }
            else
            {
                float remapped = (normalized - deadZone) / (1f - deadZone);
                remapped = Mathf.Pow(remapped, responseExponent);
                Output = (offset / distance) * remapped;
            }

            if (inputChannel != null) inputChannel.SetTouchMove(Output);
        }

        private void ResetStick()
        {
            _pointerId = NoPointer;
            Output = Vector2.zero;
            _targetAlpha = idleAlpha;
            if (inputChannel != null) inputChannel.SetTouchMove(Vector2.zero);
            if (returnToDefaultOnRelease && background != null)
                background.anchoredPosition = _defaultBackgroundPos;
        }

        private void Update()
        {
            float dt = Time.unscaledDeltaTime;

            visualGroup.alpha = Mathf.Lerp(visualGroup.alpha, _targetAlpha, 1f - Mathf.Exp(-fadeSpeed * dt));

            _popScale = Mathf.Lerp(_popScale, 1f, 1f - Mathf.Exp(-14f * dt));
            background.localScale = new Vector3(_popScale, _popScale, 1f);

            if (_pointerId == NoPointer)
                handle.anchoredPosition = Vector2.Lerp(handle.anchoredPosition, Vector2.zero, 1f - Mathf.Exp(-handleReturnSpeed * dt));
        }

        private bool ScreenToLocal(Vector2 screenPos, out Vector2 local)
        {
            Camera cam = _canvas.renderMode == RenderMode.ScreenSpaceOverlay ? null : _canvas.worldCamera;
            return RectTransformUtility.ScreenPointToLocalPointInRectangle(_area, screenPos, cam, out local);
        }
    }
}
