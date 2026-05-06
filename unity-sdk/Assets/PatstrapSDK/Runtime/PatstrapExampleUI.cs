using UnityEngine;
using UnityEngine.UI;

namespace Patstrap
{
    /// <summary>
    /// Example
    public class PatstrapExampleUI : MonoBehaviour
    {
        [SerializeField] private Slider sliderStrength;

        public void SendHaptic(string name)
        {
            PatstrapSdk.SendHaptic(name, sliderStrength.value, 1);
        }
    }
}