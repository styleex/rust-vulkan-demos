#version 450
#define SHADOW_MAP_CASCADE_COUNT 4
#define DEBUG true
#define USE_PCF true

// The `color_input` parameter of the `draw` method.
layout(set = 0, binding = 0) uniform sampler2DMS samplerAlbedo;
layout(set = 0, binding = 1) uniform sampler2DMS samplerPosition;
layout(set = 0, binding = 2) uniform sampler2DMS samplerNormal;

layout(set = 0, binding = 3) uniform sampler2DArray shadowMap;

layout(binding = 4) uniform UniformBufferObject {
    vec4 cascadeSplits;
    mat4 view;
    mat4 cascadeVP[SHADOW_MAP_CASCADE_COUNT];
} ubo;

layout(location = 0) out vec4 outFragcolor;
layout(constant_id = 0) const int NUM_SAMPLES = 2;

layout (location = 0) in vec2 inUV;


const mat4 biasMat = mat4(
0.5, 0.0, 0.0, 0.0,
0.0, 0.5, 0.0, 0.0,
0.0, 0.0, 1.0, 0.0,
0.5, 0.5, 0.0, 1.0
);

vec4 resolve(sampler2DMS tex, ivec2 uv)
{
    vec4 result = vec4(0.0);
    for (int i = 0; i < NUM_SAMPLES; i++)
    {
        vec4 val = texelFetch(tex, uv, i);
        result += val;
    }

    return result / float(NUM_SAMPLES);
}


float textureProj(vec4 posInLightView, vec2 offset, uint cascadeIndex) {
    float shadow = 1.0;
    float bias = 0.005;
    float ambient = 0.3;

    if (posInLightView.z > -1.0 && posInLightView.z < 1.0) {
        float dist = texture(shadowMap, vec3(posInLightView.st + offset, cascadeIndex)).r;

        if (posInLightView.w > 0 && dist < posInLightView.z - bias) {
            shadow = ambient;
        }
    }
    return shadow;

}


vec3 calculateLighting(vec3 pos, vec3 normal, vec4 albedo)
{
    if (normal == vec3(0.0)) {
        return albedo.rgb;
    }
    float light_percent = dot(vec3(0.7, 0.25, -0.67), normal);
    light_percent = max(light_percent, 0.0);

    return albedo.rgb * 1.5 * light_percent;
}

float filterPCF(vec4 posInLightView, uint cascadeIndex)
{
    ivec2 texDim = textureSize(shadowMap, 0).xy;
    float scale = 0.75;
    float dx = scale * 1.0 / float(texDim.x);
    float dy = scale * 1.0 / float(texDim.y);

    float shadowFactor = 0.0;
    int count = 0;
    int range = 2;

    for (int x = -range; x <= range; x++) {
        for (int y = -range; y <= range; y++) {
            shadowFactor += textureProj(posInLightView, vec2(dx*x, dy*y), cascadeIndex);
            count++;
        }
    }
    return shadowFactor / count;
}


void main() {
    ivec2 attDim = textureSize(samplerAlbedo);
    ivec2 UV = ivec2(inUV * attDim);

    // Ambient part
    vec4 alb = resolve(samplerAlbedo, UV);
    vec3 fragColor = vec3(0.0);
    float shadow = 0.0;

    // Calualte lighting for every MSAA sample
    for (int i = 0; i < NUM_SAMPLES; i++)
    {
        vec3 pos = texelFetch(samplerPosition, UV, i).rgb;

        vec3 normal = texelFetch(samplerNormal, UV, i).rgb;
        vec4 albedo = texelFetch(samplerAlbedo, UV, i);

        vec3 outSampleColor = calculateLighting(pos, normal, albedo);

        vec3 view_pos = (ubo.view * vec4(pos, 1.0)).xyz;

        // Get cascade index for the current fragment's view position
        uint shadowCascadeIndex = 0;
        for (uint i = 0; i < SHADOW_MAP_CASCADE_COUNT - 1; ++i) {
            if (view_pos.z < ubo.cascadeSplits[i]) {
                shadowCascadeIndex = i + 1;
            }
        }

        if (DEBUG) {
            switch (shadowCascadeIndex) {
                case 0 :
                outSampleColor.rgb *= vec3(1.0f, 0.25f, 0.25f);
                break;
                case 1 :
                outSampleColor.rgb *= vec3(0.25f, 1.0f, 0.25f);
                break;
                case 2 :
                outSampleColor.rgb *= vec3(0.25f, 0.25f, 1.0f);
                break;
                case 3 :
                outSampleColor.rgb *= vec3(1.0f, 1.0f, 0.25f);
                break;
            }
        }
        fragColor += outSampleColor;

        vec4 posInLightView = (biasMat * ubo.cascadeVP[shadowCascadeIndex]) * vec4(pos, 1.0);
        posInLightView /= posInLightView.w;

        if (USE_PCF) {
            shadow += filterPCF(posInLightView, shadowCascadeIndex);
        } else {
            shadow += textureProj(posInLightView, vec2(0.0), shadowCascadeIndex);
        }
    }

    shadow /= NUM_SAMPLES;
    fragColor = (alb.rgb * vec3(0.4)) + fragColor / float(NUM_SAMPLES);

    outFragcolor = vec4(fragColor, 1.0) * shadow;
}
