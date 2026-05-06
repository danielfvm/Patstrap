using UnityEngine;
using UnityEditor;
using System.Collections.Generic;


namespace Patstrap
{
    [CustomEditor(typeof(PatstrapSdkManager))]
    public class PatstrapSdkManagerInspector : Editor
    {
        // Dynamic list of haptics
        private List<Haptic> haptics = new List<Haptic>
        {
            new Haptic("left"),
            new Haptic("right"),
        };

        public override void OnInspectorGUI()
        {
            GUIStyle titleStyle = new GUIStyle(EditorStyles.boldLabel);
            titleStyle.alignment = TextAnchor.MiddleCenter;
            titleStyle.fontSize = 18;

            GUILayout.Label("Patstrap SDK", titleStyle);

            GUILayout.Space(10);

            DrawConnectionButton();

            GUILayout.Space(10);

            if (PatstrapSdk.Connected)
            {
                DrawHapticsList();
            }
        }

        private void DrawConnectionButton()
        {
            string label = PatstrapSdk.Connected ? "Disconnect" : "Connect";

            if (GUILayout.Button(label, GUILayout.Height(40)))
            {
                if (PatstrapSdk.Connected)
                {
                    PatstrapSdk.Disconnect();
                }
                else
                {
                    PatstrapSdk.Connect();
                }
            }
        }

        private void DrawHapticsList()
        {
            EditorGUILayout.LabelField("Haptics", EditorStyles.boldLabel);
            GUILayout.Space(5);

            foreach (var haptic in haptics)
            {
                EditorGUILayout.BeginHorizontal("box");

                // Name
                GUILayout.Label(haptic.Name, GUILayout.Width(120));

                // Strength slider
                haptic.Strength = EditorGUILayout.Slider(haptic.Strength, 0f, 1f);

                // Test button
                if (GUILayout.Button("Test", GUILayout.Width(60)))
                {
                    PatstrapSdk.SendHaptic(haptic.Name, haptic.Strength, 1);
                }

                EditorGUILayout.EndHorizontal();
            }
        }

        private class Haptic
        {
            public string Name;
            public float Strength;

            public Haptic(string name)
            {
                Name = name;
                Strength = 1f;
            }
        }
    }
}