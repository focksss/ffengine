#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

#define KERNAL_SIZE 16

layout (location = 0) out vec4 fragColor;

layout (location = 0) in vec2 uv;

layout(set = 0, binding = 0) uniform sampler2D g_info;
layout(set = 0, binding = 1) uniform sampler2D tex_noise;

layout(binding = 2) uniform UniformBuffer {
    vec3[KERNAL_SIZE] samples;
    mat4 projection;
    mat4 inverse_projection;
    float radius;
    int width;
    int height;
} ubo;

vec3 get_position_from_depth(vec2 in_uv) {
    float z = texture(g_info, in_uv).a;
    float x = in_uv.x * 2.0 - 1.0;
    float y = in_uv.y * 2.0 - 1.0;

    vec4 projected_position = vec4(x, y, z, 1.0);

    vec4 view_space_position = ubo.inverse_projection * projected_position;

    return view_space_position.xyz / view_space_position.w;
}

vec2 rand2(vec2 co) {
    float a = fract(sin(dot(co, vec2(12.9898, 78.233))) * 43758.5453);
    float b = fract(sin(dot(co, vec2(39.3468, 11.1357))) * 96321.1974);
    return vec2(a, b);
}

void main() {
    // fragColor = vec4(1.0, 0.0, 1.0, 1.0); return;
    vec3 normal = normalize((texture(g_info, uv).rgb - 0.5) * 2.0);

    vec2 noiseScale = vec2(float(ubo.width) / 4.0, float(ubo.height) / 4.0);
    float bias = 0.001;

    vec3 fragPos = get_position_from_depth(uv);

    // vec3 randomVec = vec3(texture(tex_noise, uv * noiseScale).xy, 0.0);
    vec3 randomVec = vec3(2.0 * rand2(uv) - 1.0, 0.0);

    vec3 tangent = normalize(randomVec - normal * dot(randomVec, normal));
    vec3 bitangent = cross(normal, tangent);
    mat3 TBN = mat3(tangent, bitangent, normal);

    //  fragColor = vec4(TBN * vec3(0.0, 0.0, 1.0), 1.0); return;

    float occlusion = 0.0;
    for(int i = 0; i < KERNAL_SIZE; i++) {
        vec3 samplePos = TBN * ubo.samples[i];
        float depthScale = abs(fragPos.z) / 5.0; // 5 is arbitrary
        float scaledRadius = ubo.radius * max(depthScale, 1.0);
        samplePos = fragPos + samplePos * scaledRadius;

        vec4 offset = vec4(samplePos, 1.0);
        offset = ubo.projection * offset;
        offset.xyz /= offset.w;
        offset.xyz = offset.xyz * 0.5 + 0.5;

        float sampleDepth = get_position_from_depth(offset.xy).z;

        float deltaDepth = abs(fragPos.z - sampleDepth);
        float rangeCheck = smoothstep(scaledRadius, 0.0, deltaDepth);
        occlusion += (sampleDepth >= samplePos.z + bias ? 1.0 : 0.0) * rangeCheck;
    }

    occlusion = 1.0 - ((occlusion) / KERNAL_SIZE);

    fragColor = vec4(vec3(occlusion), 1.0);
}