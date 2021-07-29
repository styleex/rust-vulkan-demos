#version 450

#extension GL_ARB_separate_shader_objects : enable

layout(binding = 1) uniform sampler2D texSampler;

layout(location = 0) in vec3 fragColor;
layout(location = 1) in vec2 fragTexCoord;
layout(location = 2) in vec4 fragPosition;
layout(location = 3) in vec3 fragNormal;

layout(location = 0) out vec4 outColor;
layout(location = 1) out vec4 outPosition;
layout(location = 2) out vec4 outNormal;

void main() {
    outColor = texture(texSampler, fragTexCoord); // fragPosition; //vec4(fragNormal, 1.0); //texture(texSampler, fragTexCoord);
    outPosition = fragPosition;
    outNormal = vec4(fragNormal, 1.0);
}
