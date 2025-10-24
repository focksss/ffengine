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
    int glyph_size;
    float distance_range;
} ubo;

float median(float r, float g, float b) {
    return max(min(r, g), min(max(r, g), b));
}

void main() {
    vec3 msd = texture(atlas, uv).rgb;
    float sd = median(msd.r, msd.g, msd.b);

    float screen_px_range = ubo.distance_range * float(ubo.resolution.y) / float(ubo.glyph_size);
    float screen_px_distance = screen_px_range * (sd - 0.5);

    float opacity = clamp(screen_px_distance + 0.5, 0.0, 1.0);
    frag_color = mix(vec4(0.0), color, opacity);
}
