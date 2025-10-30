#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

layout (location = 0) out vec4 blurred;

layout (location = 0) in vec2 uv;

layout(set = 0, binding = 0) uniform sampler2D color; // low res input
layout(set = 0, binding = 1) uniform sampler2D g_info; // low res

layout(set = 0, binding = 2) uniform sampler2D g_normal; // full res
layout(set = 0, binding = 3) uniform sampler2D g_depth; // full res

layout(push_constant) uniform constants {
    int horizontal;
    int radius;
    float near;
    float sigma_spatial;
    float sigma_depth;
    float sigma_normal;
    int infinite_reverse_depth;
} pc;

float get_view_z(float depth) {
    if (pc.infinite_reverse_depth == 1) {
        return pc.near / depth;
    }
    return depth;
}

float gauss(float x, float sigma) { return exp(-0.5 * (x*x) / (sigma*sigma)); }

void main() {
    ivec2 full_size = textureSize(g_depth, 0);
    ivec2 low_size = textureSize(color, 0);

    vec2 uv_low = uv * vec2(full_size) / vec2(low_size);
    vec2 texel_size_low = 1.0 / vec2(low_size);

    float center_depth = texture(g_depth, uv).r;
    vec3 center_norm = normalize(texture(g_normal, uv).rgb);
    float center_z = get_view_z(center_depth);

    vec3 accum = vec3(0.0);
    float w_sum = 0.0;

    // 2D kernel (e.g., 5x5)
    for (int y = -pc.radius; y <= pc.radius; ++y) {
        for (int x = -pc.radius; x <= pc.radius; ++x) {
            vec2 offset = vec2(x, y) * texel_size_low;
            vec2 sample_uv_low = uv_low + offset;
            vec2 sample_uv_full = sample_uv_low * vec2(low_size) / vec2(full_size);

            vec4 sample_color = texture(color, sample_uv_low);
            float sample_depth = texture(g_depth, sample_uv_full).r;
            vec3 sample_norm = normalize(texture(g_normal, sample_uv_full).rgb);
            float sample_z = get_view_z(sample_depth);

            float dist = length(vec2(x, y));
            float w_spatial = gauss(dist, pc.sigma_spatial);
            float w_depth = gauss(center_z - sample_z, pc.sigma_depth);
            float w_normal = gauss(1.0 - dot(center_norm, sample_norm), pc.sigma_normal);

            float w = w_spatial * w_depth * w_normal;
            accum += sample_color.rgb * w;
            w_sum += w;
        }
    }

    blurred = vec4(accum / w_sum, 1.0);
}