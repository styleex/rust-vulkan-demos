#version 450

// The `color_input` parameter of the `draw` method.
layout(set = 0, binding = 0) uniform sampler2DMS u_diffuse;
layout(set = 0, binding = 1) uniform sampler2DMS u_positions;
layout(set = 0, binding = 2) uniform sampler2DMS u_normals;

struct Light {
    vec3 position;
    vec3 color;
	float radius;
};

layout(set = 0, binding = 3) uniform LightData {
    Light lights[2];
	vec4 view_pos;
    int light_count;
} light_data;

layout(location = 0) out vec4 outFragcolor;
layout (constant_id = 0) const int NUM_SAMPLES = 8;

layout (location = 1) in vec2 inUV;


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


vec3 calculateLighting(vec3 pos, vec3 normal, vec4 albedo)
{
	vec3 result = vec3(0.0);

	for(int i = 0; i < light_data.light_count; ++i)
	{
		// Vector to light
		vec3 L = light_data.lights[i].position.xyz - pos;
		// Distance from light to fragment position
		float dist = length(L);

		// Viewer to fragment
		vec3 V = light_data.view_pos.xyz - pos;
		V = normalize(V);

		// Light to fragment
		L = normalize(L);

		// Attenuation
		float atten = light_data.lights[i].radius / (pow(dist, 2.0) + 1.0);

		// Diffuse part
		vec3 N = normalize(normal);
		float NdotL = max(0.0, dot(N, L));
		vec3 diff = light_data.lights[i].color * albedo.rgb * NdotL * atten;

		// Specular part
		vec3 R = reflect(-L, N);
		float NdotR = max(0.0, dot(R, V));
		vec3 spec = light_data.lights[i].color * albedo.a * pow(NdotR, 8.0) * atten;

		result += diff + spec;
	}
	return result;
}

void main() {
	ivec2 UV = ivec2(gl_FragCoord.xy);


	#define ambient 0.15

	vec4 alb = resolve(u_diffuse, UV);
	vec3 fragColor = vec3(0.0);

	for (int i = 0; i < NUM_SAMPLES; i++)
	{
		vec3 pos = texelFetch(u_positions, UV, i).rgb;
		vec3 normal = texelFetch(u_normals, UV, i).rgb;
		vec4 albedo = texelFetch(u_diffuse, UV, i);
		fragColor += calculateLighting(pos, normal, albedo);
	}

    fragColor = (alb.rgb * ambient) + fragColor / float(NUM_SAMPLES);
	outFragcolor = vec4(fragColor, 1.0);
}
