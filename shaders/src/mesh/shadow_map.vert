#version 450
#extension GL_ARB_separate_shader_objects : enable

layout (location = 0) in vec3 inPos;
layout (location = 2) in vec2 inUV;

#define SHADOW_MAP_CASCADE_COUNT 4

layout(binding = 0) uniform UniformBufferObject {
    mat4[SHADOW_MAP_CASCADE_COUNT] cascadeViewProjMat;
} ubo;

out gl_PerVertex {
    vec4 gl_Position;
};
layout (location = 0) out vec2 outUV;

void main() {
	outUV = inUV;
	gl_Position =  ubo.cascadeViewProjMat[0] * vec4(inPos, 1.0);
}
