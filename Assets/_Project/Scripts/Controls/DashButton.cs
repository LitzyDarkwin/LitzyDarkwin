using Roguelite.Core.Events;
using Roguelite.Core.Utils;
using UnityEngine;
using UnityEngine.EventSystems;
using UnityEngine.UI;

namespace Roguelite.Controls
{
    /// <summary>
    /// GameObject: Canvas altındaki "DashButton" objesine ekle (sağ alt köşe, ~200x200).
    ///
    /// Hiyerarşi:
    ///   DashButton (Image: buton arka planı, Raycast Target AÇIK)   &lt;-- BU SCRIPT
    ///    ├─ Icon          (Image: dash ikonu)                        -> iconImage
    ///    ├─ CooldownFill  (Image Type=Filled, Radial 360, siyah %60) -> cooldownFill
    ///    └─ ChargesText   (Text, opsiyonel: "2")                     -> chargesText
    ///
    /// UI ile oyuncu arasında doğrudan referans YOK:
    ///  - Basınca InputChannel.RequestDash()
    ///  - Bekleme süresini EventBus&lt;DashChargesChangedEvent&gt; ile öğrenip kendisi animasyonlar.
    /// </summary>
    public sealed class DashButton : MonoBehaviour, IPointerDownHandler
    {
        [SerializeField] private InputChannelSO inputChannel;
        [SerializeField] private Image iconImage;
        [SerializeField] private Image cooldownFill;
        [SerializeField] private Text chargesText;

        [Header("Juice")]
        [SerializeField] private SpringConfig scaleSpring = new SpringConfig(6f, 0.35f);
        [SerializeField] private float pressPunch = -0.25f;
        [SerializeField] private float readyPunch = 0.3f;
        [SerializeField] private Color readyColor = Color.white;
        [SerializeField] private Color emptyColor = new Color(1f, 1f, 1f, 0.4f);

        private SpringFloat _scale = new SpringFloat(1f);
        private int _charges = 1;
        private int _maxCharges = 1;
        private float _rechargeEndTime;
        private float _rechargeDuration;

        private void OnEnable()
        {
            EventBus<DashChargesChangedEvent>.Subscribe(OnChargesChanged);
            _scale.Reset(1f);
            Refresh();
        }

        private void OnDisable()
        {
            EventBus<DashChargesChangedEvent>.Unsubscribe(OnChargesChanged);
        }

        public void OnPointerDown(PointerEventData eventData)
        {
            if (inputChannel != null) inputChannel.RequestDash();
            _scale.AddDisplacementImpulse(pressPunch, scaleSpring);
        }

        private void OnChargesChanged(DashChargesChangedEvent e)
        {
            bool gainedCharge = e.Current > _charges;
            _charges = e.Current;
            _maxCharges = e.Max;
            _rechargeDuration = e.RechargeDuration;
            _rechargeEndTime = Time.time + e.RechargeRemaining;

            if (gainedCharge)
                _scale.AddDisplacementImpulse(readyPunch, scaleSpring); // "hazırım!" zıplaması

            Refresh();
        }

        private void Update()
        {
            _scale.Step(1f, scaleSpring, Time.unscaledDeltaTime);
            float s = _scale.Value;
            transform.localScale = new Vector3(s, s, 1f);

            if (cooldownFill != null)
            {
                if (_charges < _maxCharges && _rechargeDuration > 0f)
                {
                    float remaining = Mathf.Max(0f, _rechargeEndTime - Time.time);
                    // Hiç şarj yoksa dolu karartma, varsa sadece ince gösterge mantığıyla aynı fill
                    cooldownFill.fillAmount = remaining / _rechargeDuration;
                    cooldownFill.enabled = true;
                }
                else
                {
                    cooldownFill.enabled = false;
                }
            }
        }

        private void Refresh()
        {
            if (iconImage != null) iconImage.color = _charges > 0 ? readyColor : emptyColor;
            if (chargesText != null)
            {
                chargesText.enabled = _maxCharges > 1;
                chargesText.text = _charges.ToString();
            }
        }
    }
}
