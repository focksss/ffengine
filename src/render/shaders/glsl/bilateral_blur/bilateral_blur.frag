#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

layout (location = 0) out vec4 uFragColor;

layout (location = 0) in vec2 uv;

layout(set = 0, binding = 0) uniform sampler2D color;
layout(set = 0, binding = 1) uniform sampler2D depth;
layout(set = 0, binding = 2) uniform sampler2D view_normal;

layout(push_constant) uniform constants {
    int horizontal;
    int radius;          // kernel radius
    float nearPlane;     // camera near
    float sigmaSpatial;  // texel-space sigma
    float sigmaDepth;    // view-space sigma
    float sigmaNormal;   // normal dot sigma
    vec2 invResolution;  // 1.0 / framebuffer size
    int infinite_reverse_depth;
} pc;

float get_view_z(float depth, float near) {
    if (pc.infinite_reverse_depth == 1) {
        return near / depth;
    }
    return depth;
}

float gauss(float x, float sigma) { return exp(-0.5 * (x*x) / (sigma*sigma)); }

void main() {
    vec4 center_color = texture(color,  uv);
    float center_depth = texture(depth,  uv).r;
    vec3 center_norm = normalize(texture(view_normal, uv).xyz);
    float center_z = get_view_z(center_depth, pc.nearPlane);

    vec2 dir = pc.horizontal == 1 ? vec2(1.0, 0.0) : vec2(0.0, 1.0);

    vec3 accum = center_color.rgb;
    float w_sum = 1.0;

    for (int i = 1; i <= pc.radius; ++i) {
        vec2 offset = dir * float(i) * pc.invResolution;

        for (int side = -1; side <= 1; side += 2) {
            vec2 sample_uv = uv + offset * float(side);
            vec4 sample_col = texture(color, sample_uv);
            float sample_depth = texture(depth, sample_uv).r;
            float sample_z = get_view_z(sample_depth, pc.nearPlane);
            vec3 sample_norm = normalize(texture(view_normal, sample_uv).xyz);

            float w_spatial = gauss(float(i), pc.sigmaSpatial);
            float w_depth   = gauss(center_z - sample_z, pc.sigmaDepth);
            float n_dot     = clamp(dot(center_norm, sample_norm), -1.0, 1.0);
            float w_normal  = gauss(1.0 - n_dot, pc.sigmaNormal);

            float w = w_spatial * w_depth * w_normal;
            accum += sample_col.rgb * w;
            w_sum  += w;
        }
    }

    uFragColor = vec4(accum / w_sum, center_color.a);
}