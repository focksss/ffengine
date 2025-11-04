#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

layout (location = 0) out vec4 blurred;

layout (location = 0) in vec2 uv;

layout(set = 0, binding = 0) uniform sampler2D color; // low res input
layout(set = 0, binding = 1) uniform sampler2D g_info; // low res normal in rgb, depth in a

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
    vec2 texel_size = 1.0 / vec2(textureSize(color, 0));

    vec4 g_info_center = texture(g_info, uv);
    float center_depth = g_info_center.a;
    vec3 center_norm = normalize(g_info_center.rgb) * 2.0 - 1.0;
    float center_z = get_view_z(center_depth);

    float normal_facing = abs(center_norm.z);
    float depth_tolerance = 1.0 - abs(dot(vec3(0.0, 0.0, 1.0), center_norm));

    vec3 accum = vec3(0.0);
    float w_sum = 0.0;

    for (int i = -10; i <= 10; ++i) {
        vec2 offset_pixels = (pc.horizontal == 1 ? vec2(i, 0) : vec2(0, i));
        vec2 offset = offset_pixels * texel_size;
        vec2 sample_uv = uv + offset;
        vec4 g_info_sample = texture(g_info, sample_uv);

        vec4 sample_color = texture(color, sample_uv);
        float sample_depth = g_info_sample.a;

        if (pc.infinite_reverse_depth == 1) {
            if (sample_depth == 0.0) continue;
        } else if (sample_depth == 1.0) continue;

        vec3 sample_norm = normalize(g_info_sample.rgb) * 2.0 - 1.0;
        float sample_z = get_view_z(sample_depth);

        float dist = length(offset_pixels);
        float w_spatial = gauss(dist, pc.sigma_spatial);

        float adaptive_sigma_depth = pc.sigma_depth * depth_tolerance;
        float w_depth = gauss(abs(center_z - sample_z), adaptive_sigma_depth);

        float w_normal = gauss(1.0 - dot(center_norm, sample_norm), pc.sigma_normal);

        float w = w_spatial * w_depth * w_normal;
        accum += sample_color.rgb * w;
        w_sum += w;
    }

    blurred = vec4(accum / max(w_sum, 1e-6), 1.0);
}

/*
void main() {
    vec2 texel_size = 1.0 / vec2(textureSize(color, 0));

    vec4 g_info_center = texture(g_info, uv);
    float center_depth = g_info_center.a;
    vec3 center_norm = normalize(g_info_center.rgb * 2.0 - 1.0);
    float center_z = get_view_z(center_depth);

    float normal_facing = abs(center_norm.z);
    float depth_tolerance = 1.0 - abs(dot(vec3(0.0, 0.0, 1.0), center_norm));

    float accum = 0.0;

    int sample_count = 0;
    vec2 offset_texel = (pc.horizontal == 1 ? vec2(texel_size.x, 0) : vec2(0, texel_size.y));
    for (int i = -20; i <= 20; i++) {
        vec2 offset = i * offset_texel;
        vec2 sample_uv = uv + offset;
        vec4 g_info_sample = texture(g_info, sample_uv);
        float sample_depth = g_info_sample.a;

        if (pc.infinite_reverse_depth == 1) {
            if (sample_depth == 0.0) continue;
        } else if (sample_depth == 1.0) continue;

        vec3 sample_norm = normalize(g_info_sample.rgb * 2.0 - 1.0);
        float sample_z = get_view_z(sample_depth);

        if (abs(center_z - sample_z) < depth_tolerance && dot(sample_norm, center_norm) > 0.8) {
            accum += texture(color, sample_uv).r;
            sample_count++;
        }
    }
    blurred = vec4(vec3(accum / float(sample_count)), 1.0);
}
*/