#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

layout (location = 0) out vec4 frag_color;

layout (location = 0) in vec2 uv;
layout (location = 1) in vec4 color;
layout(set = 0, binding = 0) uniform sampler2D atlas;

layout(push_constant) uniform constants {
    vec2 min;
    vec2 max;
    vec2 position;
    ivec2 resolution;
} ubo;

float median(float r, float g, float b) {
    return max(min(r, g), min(max(r, g), b));
}

void main() {
    vec3 msd = texture(atlas, uv).rgb;
    float sd = median(msd.r, msd.g, msd.b);

    float screen_px_range = 2.0 * float(ubo.resolution.y) / 64.0;
    float screen_px_distance = screen_px_range * (sd - 0.5);

    float opacity = clamp(screen_px_distance + 0.5, 0.0, 1.0);
    frag_color = mix(vec4(0.0), color, opacity);
}
