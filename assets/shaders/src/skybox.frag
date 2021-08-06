#version 450

#extension GL_ARB_separate_shader_objects : enable

layout(binding = 1) uniform samplerCube texSampler;

layout(location = 0) in vec3 fragColor;
layout(location = 1) in vec3 fragUVW;

layout(location = 0) out vec4 outColor;
layout(location = 1) out vec4 outPosition;
layout(location = 2) out vec4 outNormal;

void main() {
    outColor = texture(texSampler, fragUVW);
//    outColor = vec4(0.53, 0.81, 0.92, 1.0);
    outPosition = vec4(1.0);
    outNormal = vec4(0.0);
}
