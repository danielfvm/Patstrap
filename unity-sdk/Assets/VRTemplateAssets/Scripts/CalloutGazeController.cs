using System;
using Unity.XR.CoreUtils;
using UnityEngine;
using UnityEngine.Events;
using UnityEngine.Serialization;

namespace Unity.VRTemplate
{
    /// <summary>
    /// Fires events when this object is is within the field of view of the gaze transform. This is currently used to
    /// hide and show tooltip callouts on the controllers when the controllers are within the field of view.
    /// </summary>
    public class CalloutGazeController : MonoBehaviour
    {
        [SerializeField, Tooltip("The transform which the forward direction will be used to evaluate as the gaze direction.")]
        Transform m_GazeTransform;

        [SerializeField, Tooltip("Threshold for the dot product when determining if the Gaze Transform is facing this object. The lower the threshold, the wider the field of view."), Range(0.0f, 1.0f)]
        float m_FacingThreshold = 0.85f;

        [SerializeField, Tooltip("Events fired when the Gaze Transform begins facing this game object")]
        UnityEvent m_FacingEntered;

        [SerializeField, Tooltip("Events fired when the Gaze Transform stops facing this game object")]
        UnityEvent m_FacingExited;

        [SerializeField, Tooltip("Distance threshold for movement in a single frame that determines a large movement that will trigger Facing Exited events.")]
        float m_LargeMovementDistanceThreshold = 0.05f;

        [SerializeField, Tooltip("Cool down time after a large movement for Facing Entered events to fire again.")]
        float m_LargeMovementCoolDownTime = 0.25f;

        bool m_IsFacing;
        float m_LargeMovementCoolDown;
        Vector3 m_LastPosition;

        void Update()
        {
            if (!m_GazeTransform)
                return;

            CheckLargeMovement();

            if (m_LargeMovementCoolDown < m_LargeMovementCoolDownTime)
                return;

            var dotProduct = Vector3.Dot(m_GazeTransform.forward, (transform.position - m_GazeTransform.position).normalized);
            if (dotProduct > m_FacingThreshold && !m_IsFacing)
                FacingEntered();
            else if (dotProduct < m_FacingThreshold && m_IsFacing)
                FacingExited();
        }

        void CheckLargeMovement()
        {
            // Check if there is large movement
            var currentPosition = transform.position;
            var positionDelta = Mathf.Abs(Vector3.Distance(m_LastPosition, currentPosition));
            if (positionDelta > m_LargeMovementDistanceThreshold)
            {
                m_LargeMovementCoolDown = 0.0f;
                FacingExited();
            }
            m_LargeMovementCoolDown += Time.deltaTime;
            m_LastPosition = currentPosition;
        }

        void FacingEntered()
        {
            m_IsFacing = true;
            m_FacingEntered.Invoke();
        }

        void FacingExited()
        {
            m_IsFacing = false;
            m_FacingExited.Invoke();
        }
    }
}
