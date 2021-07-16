#version 450

// The `color_input` parameter of the `draw` method.
layout(set = 0, binding = 0) uniform sampler2DMS u_diffuse;


layout(location = 0) out vec4 outFragcolor;
layout(constant_id = 0) const int NUM_SAMPLES = 16;

layout (location = 0) in vec2 inUV;


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
    ivec2 attDim = textureSize(u_diffuse);
    ivec2 UV = ivec2(inUV * attDim);

    vec4 alb = resolve(u_diffuse, UV);

    outFragcolor = vec4(alb.rgb, 1.0);
}
