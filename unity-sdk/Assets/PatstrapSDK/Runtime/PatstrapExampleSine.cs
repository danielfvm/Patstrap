using System;
using UnityEngine;
using UnityEngine.UI;
using UnityEngine.XR.Interaction.Toolkit;

namespace Patstrap
{
    /// <summary>
    /// Example
    public class PatstrapExampleSine : MonoBehaviour
    {
        [SerializeField] private float delay = 0.1f;
        
        private float time;

        private void Update()
        { 
            time += Time.deltaTime;
            if (time > delay)
            {
                time -= delay;

                float s = Mathf.Sin(Time.time) * 0.5f + 0.5f;

                PatstrapSdk.SendHaptic("right", 1f - s, delay * 2.0f);
                PatstrapSdk.SendHaptic("left", s, delay * 2.0f);
            }
        }
    }
}