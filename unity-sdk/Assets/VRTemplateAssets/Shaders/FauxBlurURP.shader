Shader "SpatialFramework/FauxBackgroundOverlayBlurURP"
{
    Properties
    {
        _Blur("Blur", Range(0, 10)) = 1.5
        _Alpha("Alpha", Range(0, 1)) = 1
        _GradientSize("Gradient Size", Range(0, 6)) = 2
        _MainTex("Noise Texture (REQUIRED for Blur Noise)", 2D) = "white" {}
    }

    SubShader
    {
        PackageRequirements
        {
            "com.unity.render-pipelines.universal": "12.1.3"
        }

        Tags { "RenderPipeline" = "UniversalRenderPipeline" "Queue" = "Transparent-1" "LightMode" = "Always" "IgnoreProjector" = "True" "RenderType" = "Transparent" }

        ZWrite On
        ZTest LEqual
        Lighting Off
        Blend SrcAlpha OneMinusSrcAlpha

        Pass
        {
            Tags { "LightMode" = "UniversalForward" }

            HLSLPROGRAM
            #pragma vertex vert
            #pragma fragment frag

            #include "Packages/com.unity.render-pipelines.universal/ShaderLibrary/Core.hlsl"
            #include "Packages/com.unity.render-pipelines.universal/ShaderLibrary/ShaderVariablesFunctions.hlsl"
            #include "Packages/com.unity.render-pipelines.core/ShaderLibrary/EntityLighting.hlsl"

            SAMPLER(_MainTex);

            CBUFFER_START(UnityPerMaterial)
            half _Alpha;
            half _Blur;
            half _GradientSize;
            CBUFFER_END

            struct appdata_t
            {
                half4 position : POSITION;
                half3 normal : NORMAL;
                half2 texcoord : TEXCOORD0;
                UNITY_VERTEX_INPUT_INSTANCE_ID
            };

            struct v2f
            {
                half4 position : POSITION;
                half3 worldPos : TEXCOORD0;
                half2 cleanUV : TEXCOORD2;
                UNITY_VERTEX_OUTPUT_STEREO
            };

            v2f vert(appdata_t v)
            {
                v2f output;
                UNITY_SETUP_INSTANCE_ID(v);
                UNITY_INITIALIZE_VERTEX_OUTPUT_STEREO(output);

                output.position = TransformObjectToHClip(v.position.xyz);
                output.worldPos = mul(unity_ObjectToWorld, v.position).xyz;
                output.cleanUV = v.texcoord;

                return output;
            }

            half4 frag(v2f input) : SV_Target
            {
                half2 uvPos = abs(input.cleanUV - float2(0.5, 0.5));
                half uvMax = max(uvPos.x, uvPos.y);
                half fadeFromBorderAmount = 1 - clamp(0, 1, pow(uvMax, _GradientSize) * 2);
                half3 reflectionDir = -normalize(GetWorldSpaceViewDir(input.worldPos));
                half noise = tex2D(_MainTex, input.cleanUV).r;
                half4 reflectionData = SAMPLE_TEXTURECUBE_LOD(unity_SpecCube0, samplerunity_SpecCube0, reflectionDir, 1.5);
                half3 reflectionColor = DecodeHDREnvironment(reflectionData, unity_SpecCube0_HDR);
                return half4(reflectionColor, clamp(0, 1 - pow((uvMax * 2), _GradientSize * (_Blur / 10)), fadeFromBorderAmount) * _Alpha * noise);
            }
            ENDHLSL
        }
    }

    SubShader
    {
        Tags{ "RenderPipeline" = " " "Queue" = "Transparent-1" "LightMode" = "Always" "IgnoreProjector" = "True" "RenderType" = "Transparent" }

        ZWrite On
        ZTest LEqual
        Lighting Off
        Blend SrcAlpha OneMinusSrcAlpha

        Pass
        {
            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #include "UnityCG.cginc"

            half _Alpha;
            half _Blur;
            half _GradientSize;
            sampler2D _MainTex;

            struct appdata_t
            {
                half4 position : POSITION;
                half3 normal : NORMAL;
                half2 texcoord : TEXCOORD0;
                UNITY_VERTEX_INPUT_INSTANCE_ID
            };

            struct v2f
            {
                half4 position : POSITION;
                half3 worldPos : TEXCOORD0;
                half2 cleanUV : TEXCOORD2;
                UNITY_VERTEX_OUTPUT_STEREO
            };

            v2f vert(appdata_t v)
            {
                v2f output;
                UNITY_SETUP_INSTANCE_ID(v);
                UNITY_INITIALIZE_VERTEX_OUTPUT_STEREO(output);

                output.position = UnityObjectToClipPos(v.position);
                output.worldPos = mul(unity_ObjectToWorld, v.position).xyz;
                output.cleanUV = v.texcoord;

                return output;
            }

            fixed4 frag(v2f input) : SV_Target
            {
                half2 uvPos = abs(input.cleanUV - float2(0.5, 0.5));
                half uvMax = max(uvPos.x, uvPos.y);
                half fadeFromBorderAmount = 1 - clamp(0, 1, pow(uvMax, _GradientSize) * 2);
                half3 reflectionDir = -normalize(UnityWorldSpaceViewDir(input.worldPos));
                half noise = tex2D(_MainTex, input.cleanUV).r;
                half4 reflectionData = UNITY_SAMPLE_TEXCUBE_LOD(unity_SpecCube0, reflectionDir, 1.5);
                half3 reflectionColor = DecodeHDR(reflectionData, unity_SpecCube0_HDR);
                return half4(reflectionColor, clamp(0, 1 - pow((uvMax * 2), _GradientSize * (_Blur / 10)), fadeFromBorderAmount) * _Alpha * noise);
            }
            ENDCG
        }
    }
}
