using System.Net;
using System.Net.Sockets;

namespace Patstrap
{
    public static class PatstrapSdk
    {
        private static UdpClient client;
        private static IPEndPoint endPoint;

        public static bool Connected => client != null;

        public static void Connect(string host = "127.0.0.1", int port = 5123)
        {
            if (Connected)
                return;

            client = new UdpClient();
            endPoint = new IPEndPoint(IPAddress.Parse(host), port);

            Send("/connect");
        }

        public static bool SendHaptic(string name, float strength, float duration)
        {
            if (!Connected)
                return false;

            Send("/haptic", name, strength, duration);
        
            return true;
        }

        public static void Disconnect()
        {
            if (!Connected)
                return;

            Send("/disconnect");
            client?.Dispose();
            client = null;
            endPoint = null;
        }

        private static void Send(string address, params object[] args)
        {
            byte[] values = OscEncoder.Encode(address, args);
            client?.Send(values, values.Length, endPoint);
        }
    }
}