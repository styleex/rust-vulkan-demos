#version 450

layout(location = 0) in vec4 inColor;
layout(location = 1) in vec2 inUV;

layout(location = 0) out vec4 outColor;

layout(binding = 0, set = 0) uniform sampler2DMS user_texture;

layout(constant_id = 0) const int NUM_SAMPLES = 8;


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


void main() {
    ivec2 attDim = textureSize(user_texture);
    ivec2 UV = ivec2(inUV * attDim);

    outColor = inColor * resolve(user_texture, UV);
}
