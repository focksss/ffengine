#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

layout (location = 0) out vec4 frag_color;

layout (location = 0) in vec2 uv;
layout (location = 1) in vec2 pos;

layout(set = 0, binding = 0) uniform sampler2D textures[];

layout(push_constant) uniform constants {
    vec4 additive_color;
    vec4 multiplicative_color;
    ivec2 resolution;
    vec2 min;
    vec2 max;
    vec2 position;
    vec2 scale;
    float corner_radius;
    int image;
} ubo;

void main() {
    vec2 frag_pos = vec2(ubo.resolution) * pos;

    if (frag_pos.x < ubo.min.x || frag_pos.x > ubo.max.x ||
    frag_pos.y < ubo.min.y || frag_pos.y > ubo.max.y)
    discard;

    float radius = ubo.corner_radius;
    vec2 rect_min = ubo.position;
    vec2 rect_max = ubo.position + ubo.scale;
    vec2 inner_min = rect_min + vec2(radius);
    vec2 inner_max = rect_max - vec2(radius);

    vec2 nearest = clamp(frag_pos, inner_min, inner_max);
    float dist = length(frag_pos - nearest);

    if (dist > radius)
    discard;

    frag_color = ubo.additive_color;
    if (ubo.image > -1) {
        vec4 tex_col = texture(textures[ubo.image], uv);
        frag_color += tex_col;
    }
    frag_color *= ubo.multiplicative_color;
}
