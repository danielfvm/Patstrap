using System;
using System.Collections.Generic;
using System.Net;
using System.Text;

namespace Patstrap
{
    public static class OscEncoder
    {
        public static byte[] Encode(string address, params object[] args)
        {
            if (string.IsNullOrEmpty(address) || !address.StartsWith("/"))
                throw new ArgumentException("OSC address must start with '/'");

            List<byte> data = new List<byte>();

            // Address
            data.AddRange(PadOscString(address));

            // Type tag
            string typeTag = ",";
            List<byte[]> argumentBytes = new List<byte[]>();

            foreach (var arg in args)
            {
                switch (arg)
                {
                    case int i:
                        typeTag += "i";
                        argumentBytes.Add(IntToBytes(i));
                        break;

                    case float f:
                        typeTag += "f";
                        argumentBytes.Add(FloatToBytes(f));
                        break;

                    case string s:
                        typeTag += "s";
                        argumentBytes.Add(PadOscString(s));
                        break;

                    case bool b:
                        typeTag += b ? "T" : "F"; // no data payload
                        break;

                    default:
                        throw new NotSupportedException($"Unsupported type: {arg.GetType()}");
                }
            }

            // Add type tag
            data.AddRange(PadOscString(typeTag));

            // Add argument data
            foreach (var bytes in argumentBytes)
            {
                data.AddRange(bytes);
            }

            return data.ToArray();
        }

        private static byte[] PadOscString(string value)
        {
            byte[] strBytes = Encoding.ASCII.GetBytes(value);
            int lenWithNull = strBytes.Length + 1;

            int paddedLength = (lenWithNull + 3) & ~3; // align to 4 bytes
            byte[] result = new byte[paddedLength];

            Array.Copy(strBytes, result, strBytes.Length);
            // null terminator already zero

            return result;
        }

        private static byte[] IntToBytes(int value)
        {
            return BitConverter.GetBytes(IPAddress.HostToNetworkOrder(value));
        }

        private static byte[] FloatToBytes(float value)
        {
            byte[] bytes = BitConverter.GetBytes(value);

            if (BitConverter.IsLittleEndian)
                Array.Reverse(bytes);

            return bytes;
        }
    }
}