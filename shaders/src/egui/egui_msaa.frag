#version 450

layout(location = 0) in vec4 inColor;
layout(location = 1) in vec2 inUV;

layout(location = 0) out vec4 outColor;

layout(binding = 0, set = 0) uniform sampler2DMS font_texture;

void main() {
    ivec2 attDim = textureSize(font_texture);
    ivec2 UV = ivec2(inUV * attDim);

    outColor = inColor * texelFetch(font_texture, UV, 0);
}
