#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

layout (location = 0) out vec4 blurred;

layout (location = 0) in vec2 uv;

layout(set = 0, binding = 0) uniform sampler2D color;
layout(set = 0, binding = 1) uniform sampler2D g_info;

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
        return pc.near / (1.0 - depth);
    }
    return depth;
}

float gauss(float x, float sigma) { return exp(-0.5 * (x*x) / (sigma*sigma)); }

void main() {
    // uFragColor = vec4(1.0, 0.0, 1.0, 1.0); return;

    vec4 center_color = texture(color,  uv);
    float center_depth = texture(g_info,  uv).a;
    vec3 center_norm = normalize(texture(g_info, uv).rgb);
    float center_z = get_view_z(center_depth);

    vec2 dir = pc.horizontal == 1 ? vec2(1.0, 0.0) : vec2(0.0, 1.0);

    vec3 accum = center_color.rgb;
    float w_sum = 1.0;

    for (int i = 1; i <= pc.radius; ++i) {
        vec2 offset = dir * float(i) * (1 / vec2(textureSize(g_info, 0)));

        for (int side = -1; side <= 1; side += 2) {
            vec2 sample_uv = uv + offset * float(side);
            vec4 sample_col = texture(color, sample_uv);
            float sample_depth = texture(g_info, sample_uv).a;
            float sample_z = get_view_z(sample_depth);
            vec3 sample_norm = normalize(texture(g_info, sample_uv).rgb);

            float w_spatial = gauss(float(i), pc.sigma_spatial);
            float w_depth   = gauss(center_z - sample_z, pc.sigma_depth);
            float n_dot     = clamp(dot(center_norm, sample_norm), -1.0, 1.0);
            float w_normal  = gauss(1.0 - n_dot, pc.sigma_normal);

            float w = w_spatial * w_depth * w_normal;
            accum += sample_col.rgb * w;
            w_sum  += w;
        }
    }

    blurred = vec4(accum / w_sum, center_color.a);
}