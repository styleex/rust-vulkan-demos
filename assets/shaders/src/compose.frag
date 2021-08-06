#version 450

// The `color_input` parameter of the `draw` method.
layout(set = 0, binding = 0) uniform sampler2DMS samplerAlbedo;
layout(set = 0, binding = 1) uniform sampler2DMS samplerPosition;
layout(set = 0, binding = 2) uniform sampler2DMS samplerNormal;

layout(set = 0, binding = 3) uniform sampler2D shadowMap;

layout(binding = 4) uniform UniformBufferObject {
    mat4 light_vp;
} ubo;



layout(location = 0) out vec4 outFragcolor;
layout(constant_id = 0) const int NUM_SAMPLES = 8;

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


float textureProj(vec4 shadowCoord, vec2 offset, uint cascadeIndex) {
	float shadow = 1.0;
	float bias = 0.005;
	float ambient = 0.3;

	if ( shadowCoord.z > -1.0 && shadowCoord.z < 1.0 ) {
		float dist = texture(shadowMap, shadowCoord.st + offset).r;
		if (shadowCoord.w > 0 && dist < shadowCoord.z - bias) {
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
    float light_percent = -dot(vec3(0.1, -0.1, 0.1), normal);
    light_percent = max(light_percent, 0.0);

	return albedo.rgb * 1.5 * light_percent;
}


void main() {
    ivec2 attDim = textureSize(samplerAlbedo);
    ivec2 UV = ivec2(inUV * attDim);

	vec3 pos = resolve(samplerPosition, UV).xyz;
	vec4 shadowCoord = (biasMat * ubo.light_vp) * vec4(pos, 1.0);
	float shadow = textureProj(shadowCoord / shadowCoord.w, vec2(0.0), 0);


	// Ambient part
	vec4 alb = resolve(samplerAlbedo, UV);
	vec3 fragColor = vec3(0.0);

	// Calualte lighting for every MSAA sample
	for (int i = 0; i < NUM_SAMPLES; i++)
	{
		vec3 pos = texelFetch(samplerPosition, UV, i).rgb;
		vec3 normal = texelFetch(samplerNormal, UV, i).rgb;
		vec4 albedo = texelFetch(samplerAlbedo, UV, i);
		fragColor += calculateLighting(pos, normal, albedo);
	}

//	fragColor = resolve(samplerNormal, UV).rgb;

	fragColor = (alb.rgb * vec3(0.4)) + fragColor / float(NUM_SAMPLES);

	outFragcolor = vec4(fragColor, 1.0) * shadow;
}
