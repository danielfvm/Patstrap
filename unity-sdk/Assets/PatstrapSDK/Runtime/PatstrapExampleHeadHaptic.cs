using System;
using UnityEngine;
using UnityEngine.UI;
using UnityEngine.XR.Interaction.Toolkit;

namespace Patstrap
{
    /// <summary>
    /// Example
    public class PatstrapExampleHeadHaptic : MonoBehaviour
    {
        [SerializeField] private string hapticName;
        [SerializeField] private XRGrabInteractable pointer;
        [SerializeField] private float delay = 0.1f;
        [SerializeField] private float radius = 1f;
        
        private bool pressed;
        private float time;

        private void Start()
        {
            pointer.activated.AddListener((_) => pressed = true); 
            pointer.deactivated.AddListener((_) => pressed = false); 
        }

        private void Update()
        {
            if (pressed)
            {         
                time += Time.deltaTime;
                if (time > delay)
                {
                    time -= delay;

                    float d = Vector3.Distance(transform.position, pointer.transform.position);
                    float strength = 1.0f - Mathf.Clamp01(d / radius);

                    if (strength > 0)
                        PatstrapSdk.SendHaptic(hapticName, strength, delay * 2.0f);
                }
            }
        }

        private void OnDrawGizmos()
        {
            Gizmos.DrawWireSphere(transform.position, radius);
        }
    }
}