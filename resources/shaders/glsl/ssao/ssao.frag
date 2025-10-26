#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

#define KERNAL_SIZE 16

layout (location = 0) out vec4 fragColor;

layout (location = 0) in vec2 uv;

layout(set = 0, binding = 0) uniform sampler2D g_depth;
layout(set = 0, binding = 1) uniform sampler2D view_normal;
layout(set = 0, binding = 2) uniform sampler2D tex_noise;

layout(binding = 3) uniform UniformBuffer {
    vec3[KERNAL_SIZE] samples;
    mat4 projection;
    mat4 inverse_projection;
    float radius;
    int width;
    int height;
} ubo;

vec3 get_position_from_depth(vec2 in_uv) {
    float z = texture(g_depth, in_uv).r;
    float x = in_uv.x * 2.0 - 1.0;
    float y = in_uv.y * 2.0 - 1.0;

    vec4 projected_position = vec4(x, y, z, 1.0);

    vec4 view_space_position = ubo.inverse_projection * projected_position;

    return view_space_position.xyz / view_space_position.w;
}

void main() {
    // fragColor = vec4(1.0, 0.0, 1.0, 1.0); return;
    vec2 noiseScale = vec2(float(ubo.width) / 4.0, float(ubo.height) / 4.0);
    float bias = 0.001;

    vec3 fragPos = get_position_from_depth(uv);
    vec3 normal = normalize(texture(view_normal, uv).rgb-0.5)* ( textureSize(view_normal, 0).x / ubo.width);
    vec3 randomVec = normalize(texture(tex_noise, uv * noiseScale).xyz);

    vec3 tangent = normalize(randomVec - normal * dot(randomVec, normal));
    vec3 bitangent = cross(normal, tangent);
    mat3 TBN = mat3(tangent, bitangent, normal);

    float occlusion = 0.0;
    for(int i = 0; i < KERNAL_SIZE; i++) {
        vec3 samplePos = TBN * ubo.samples[i];
        samplePos = fragPos + samplePos * ubo.radius;

        vec4 offset = vec4(samplePos, 1.0);
        offset = ubo.projection * offset;
        offset.xyz /= offset.w;
        offset.xyz = offset.xyz * 0.5 + 0.5;

        float sampleDepth = get_position_from_depth(offset.xy).z;

        float deltaDepth = abs(fragPos.z - sampleDepth);
        if (deltaDepth < ubo.radius) {
            float rangeCheck = smoothstep(0.0, 1.0, ubo.radius / deltaDepth);
            occlusion += min((sampleDepth >= samplePos.z + 0.001 ? 1.0 : 0.0) * rangeCheck, 1.0);
        }
    }

    occlusion = 1.0 - ((occlusion) / KERNAL_SIZE);

    fragColor = vec4(vec3(occlusion), 1.0);
}