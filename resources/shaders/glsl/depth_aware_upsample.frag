#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

layout (location = 0) out vec4 upsampled;
layout (location = 0) in vec2 uv;

layout(set = 0, binding = 0) uniform sampler2D color;      // low res input (pre-blurred)
layout(set = 0, binding = 1) uniform sampler2D g_depth_low; // low res depth
layout(set = 0, binding = 2) uniform sampler2D g_normal;   // full res normal
layout(set = 0, binding = 3) uniform sampler2D g_depth;    // full res depth

layout(push_constant) uniform constants {
    float near;
    float depth_threshold; // sensitivity for depth edges (e.g., 0.01-0.1)
    float normal_threshold; // sensitivity for normal edges (e.g., 0.9)
    float sharpness; // edge sharpness multiplier (e.g., 8.0)
    int infinite_reverse_depth;
} pc;

float get_view_z(float depth) {
    if (pc.infinite_reverse_depth == 1) {
        return pc.near / depth;
    }
    return depth;
}

void main() {
    ivec2 full_size = textureSize(g_depth, 0);
    ivec2 low_size = textureSize(color, 0);
    vec2 texel_size_low = 1.0 / vec2(low_size);

    float center_depth = texture(g_depth, uv).r;
    vec3 center_norm = normalize(texture(g_normal, uv).rgb * 2.0 - 1.0);
    float center_z = get_view_z(center_depth);

    bool invalid_center = (pc.infinite_reverse_depth == 1 && center_depth == 0.0) ||
    (pc.infinite_reverse_depth == 0 && center_depth == 1.0);

    if (invalid_center) {
        upsampled = texture(color, uv);
        return;
    }

    vec3 accum = vec3(0.0);
    float w_sum = 0.0;

    for (int y = 1; y <= 2; ++y) {
        for (int x = 1; x <= 2; ++x) {
            vec2 offset = (vec2(x, y) - 0.5) * texel_size_low;
            vec2 sample_uv = uv + offset;

            vec3 sample_color = texture(color, sample_uv).rgb;
            float sample_depth = texture(g_depth_low, sample_uv).r;

            if ((pc.infinite_reverse_depth == 1 && sample_depth == 0.0) ||
            (pc.infinite_reverse_depth == 0 && sample_depth == 1.0)) {
                continue;
            }

            vec3 sample_norm = normalize(texture(g_normal, sample_uv).rgb * 2.0 - 1.0);
            float sample_z = get_view_z(sample_depth);

            float depth_diff = abs(center_z - sample_z) / max(abs(center_z), 1e-4);

            float normal_sim = dot(center_norm, sample_norm);

            vec2 frac_coord = fract(uv * vec2(low_size) - 0.5);
            float w_bilinear = (x == 0 ? (1.0 - frac_coord.x) : frac_coord.x) *
            (y == 0 ? (1.0 - frac_coord.y) : frac_coord.y);

            float w_depth = exp(-depth_diff * pc.sharpness / pc.depth_threshold);

            float w_normal = normal_sim > pc.normal_threshold ? 1.0 : 0.1;

            float w = w_bilinear * w_depth * w_normal;

            accum += sample_color * w;
            w_sum += w;
        }
    }

    if (w_sum > 1e-6) {
        upsampled = vec4(accum / w_sum, 1.0);
    } else {
        upsampled = texture(color, uv);
    }
}