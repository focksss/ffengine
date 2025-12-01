#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

layout (location = 0) out vec4 frag_color;

layout (location = 0) in vec2 uv;
layout (location = 1) in vec4 color;
layout (location = 2) in vec2 pos;

layout(set = 0, binding = 0) uniform sampler2D atlases[];

layout(push_constant) uniform constants {
    vec2 min;
    vec2 max;
    vec2 position;
    ivec2 resolution;
    int glyph_size;
    float distance_range;
    uint font_index;
    float align_shift;
    float font_size;
} ubo;

float median(float r, float g, float b) {
    return max(min(r, g), min(max(r, g), b));
}

void main() {
    vec2 frag_pos = vec2(ubo.resolution) * pos;
    if (frag_pos.x < ubo.max.x && frag_pos.x > ubo.min.x) {
        if (frag_pos.y < ubo.max.y && frag_pos.y > ubo.min.y) {

            vec3 msd = texture(atlases[ubo.font_index], uv).rgb;
            float sd = median(msd.r, msd.g, msd.b);

            float screen_px_range = ubo.distance_range * float(ubo.resolution.y) / float(ubo.glyph_size);
            float screen_px_distance = screen_px_range * (sd - 0.5);

            float w = fwidth(screen_px_distance);
            float opacity = smoothstep(-w, w, screen_px_distance);

            frag_color = mix(vec4(0.0), color, opacity);
        } else {
            discard;
        }
    } else {
        discard;
    }
}
