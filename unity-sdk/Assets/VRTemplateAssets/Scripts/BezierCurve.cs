using System;
using UnityEngine;

namespace Unity.VRTemplate
{
    /// <summary>
    /// Draws a bezier curve from a starting point transform to an end point transform
    /// </summary>
    public class BezierCurve : MonoBehaviour
    {
        /// <summary>
        /// If the view scale changes more than this amount, then the line width will be updated causing the line to be rebuilt.
        /// </summary>
        const float k_ViewerScaleChangeThreshold = 0.1f;

        /// <summary>
        /// The time within the frame that the curve will be updated.
        /// </summary>
        /// <seealso cref="UnityEngine.XR.Interaction.Toolkit.XRBaseController.UpdateType"/>
        public enum UpdateType
        {
            /// <summary>
            /// Sample at both update and directly before rendering. For smooth tracking,
            /// we recommend using this value as it will provide the lowest input latency for the device.
            /// </summary>
            UpdateAndBeforeRender,

            /// <summary>
            /// Only sample input during the update phase of the frame.
            /// </summary>
            Update,

            /// <summary>
            /// Only sample input directly before rendering.
            /// </summary>
            BeforeRender,
        }

#pragma warning disable 649
        [SerializeField, Tooltip("The time within the frame that the curve will be updated. If this Bezier Curve is attached to a transform that is updating before render, then enabling updates in Before Render will keep the line connected without delay.")]
        UpdateType m_UpdateTrackingType = UpdateType.Update;

        [SerializeField, Tooltip("The transform that determines the position, handle rotation, and handle scale of the start point of the bezier curve.")]
        Transform m_StartPoint;

        [SerializeField, Tooltip("The transform that determines the position, handle rotation, and handle scale of the end point of the bezier curve.")]
        Transform m_EndPoint;

        [SerializeField, Tooltip("Controls the scale factor of the curve's start bezier handle.")]
        float m_CurveFactorStart = 1.0f;

        [SerializeField, Tooltip("Controls the scale factor of the curve's end bezier handle.")]
        float m_CurveFactorEnd = 1.0f;

        [SerializeField, Tooltip("Controls the number of segments used to draw the curve.")]
        int m_SegmentCount = 50;

        [SerializeField, Tooltip("When enabled, the line color gradient will be animated so that an opaque part travels along the line.")]
        bool m_Animate;

        [SerializeField, Tooltip("If animated, this controls the speed that the animation of the line.")]
        float m_AnimSpeed = 0.25f;

        [SerializeField, Tooltip("If animated, this color will be the main opaque color of the gradient")]
        Color m_GradientKeyColor = new Color(0.1254902f, 0.5882353f, 0.9529412f);

        [SerializeField, Tooltip("The line renderer that will draw the curve. If not set it will find a line renderer on this GameObject.")]
        LineRenderer m_LineRenderer;
#pragma warning restore 649

        Vector3[] m_ControlPoints = new Vector3[4];
        float m_Time;
        float m_LineWidth;
        float m_LastViewerScale;

        Vector3 m_LastStartPosition;
        Vector3 m_LastEndPosition;
        //IProvidesViewerScale IFunctionalitySubscriber<IProvidesViewerScale>.provider { get; set; }

        void Awake()
        {
            if (m_LineRenderer == null)
                m_LineRenderer = GetComponent<LineRenderer>();

            m_LineWidth = m_LineRenderer.startWidth;
        }

        void OnEnable()
        {
            DrawCurve();
            Application.onBeforeRender += OnBeforeRender;
        }

        void OnDisable()
        {
            Application.onBeforeRender -= OnBeforeRender;

        }

        void OnBeforeRender()
        {
            if (m_UpdateTrackingType == UpdateType.BeforeRender || m_UpdateTrackingType == UpdateType.UpdateAndBeforeRender)
                DrawCurve();
        }

        void Update()
        {
            if (m_UpdateTrackingType == UpdateType.Update || m_UpdateTrackingType == UpdateType.UpdateAndBeforeRender)
                DrawCurve();

            if (m_Animate)
            {
                AnimateCurve();
            }
        }

        /// <summary>
        /// Updates the line points to draw the bezier curve.
        /// </summary>
        [ContextMenu("Draw")]
        public void DrawCurve()
        {
            var startPointPosition = m_StartPoint.position;
            var endPointPosition = m_EndPoint.position;

            if (startPointPosition == m_LastStartPosition &&
                endPointPosition == m_LastEndPosition)
                return; // Return early if the start and end have not changed to avoid recalculating the curve

            var dist = Vector3.Distance(startPointPosition, endPointPosition);

            m_ControlPoints[0] = startPointPosition;
            m_ControlPoints[1] = startPointPosition + (m_StartPoint.right * (dist * m_CurveFactorStart));
            m_ControlPoints[2] = endPointPosition - (m_EndPoint.right * (dist * m_CurveFactorEnd));
            m_ControlPoints[3] = endPointPosition;

            int segmentCount;
            const float smallestCurveLength = 0.0125f;
            if (Vector3.Distance(startPointPosition, endPointPosition) < (smallestCurveLength * m_LastViewerScale))
            {
                segmentCount = 2;
            }
            else
            {
                segmentCount = m_SegmentCount;
            }

            m_LineRenderer.positionCount = segmentCount + 1;
            m_LineRenderer.SetPosition(0, m_ControlPoints[0]);
            for (var i = 1; i <= segmentCount; i++)
            {
                var t = i / (float)segmentCount;
                var pixel = CalculateCubicBezierPoint(t, m_ControlPoints[0], m_ControlPoints[1], m_ControlPoints[2], m_ControlPoints[3]);
                m_LineRenderer.SetPosition(i, pixel);
            }

            m_LastStartPosition = startPointPosition;
            m_LastEndPosition = endPointPosition;
        }

        static Vector3 CalculateCubicBezierPoint(float t, Vector3 p0, Vector3 p1, Vector3 p2, Vector3 p3)
        {
            var u = 1 - t;
            var tt = t * t;
            var uu = u * u;
            var uuu = uu * u;
            var ttt = tt * t;

            var p = uuu * p0;
            p += 3 * uu * t * p1;
            p += 3 * u * tt * p2;
            p += ttt * p3;

            return p;
        }

        void AnimateCurve()
        {
            var newGrad = new Gradient();

            var colorKeys = new GradientColorKey[1];
            var alphaKeys = new GradientAlphaKey[2];

            var colorKey = new GradientColorKey(m_GradientKeyColor, 0f);
            colorKeys[0] = colorKey;

            var alphaKeyStart = new GradientAlphaKey(.25f, m_Time);
            var alphaKeyEnd = new GradientAlphaKey(1f, 1f);
            alphaKeys[0] = alphaKeyStart;
            alphaKeys[1] = alphaKeyEnd;

            newGrad.SetKeys(colorKeys, alphaKeys);
            newGrad.mode = GradientMode.Blend;

            m_LineRenderer.colorGradient = newGrad;
            m_Time += (Time.unscaledDeltaTime * m_AnimSpeed);

            if (m_Time >= 1f)
                m_Time = 0f;
        }
    }
}
