using UnityEngine;

namespace Patstrap
{
    /// <summary>
    /// The PatstrapSdkManager ensures that the PatstrapSdk is connected
    /// and automatically disconnects at start / shutdown of the game.
    /// 
    /// This Manager script is completely optionally. You can call 
    /// PatstrapSDK.Connect() and PatstrapSDK.Disconnect() instead as well.
    /// </summary>
    [DefaultExecutionOrder(-10000)]
    public class PatstrapSdkManager : MonoBehaviour
    {
        private void Awake()
        {
            PatstrapSdk.Connect();
        }

        private void OnApplicationQuit()
        {
            PatstrapSdk.Disconnect();
        }
    }
}