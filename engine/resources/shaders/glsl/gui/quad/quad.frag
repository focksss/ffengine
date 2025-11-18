#version 460

layout (location = 0) out vec4 frag_color;

layout (location = 0) in vec2 uv;
layout (location = 1) in vec2 pos;

layout(push_constant) uniform constants {
    vec4 color;
    ivec2 resolution;
    vec2 min;
    vec2 max;
    vec2 position;
    vec2 scale;
    float corner_radius;
} ubo;

void main() {
    vec2 frag_pos = vec2(ubo.resolution) * pos;

    if (frag_pos.x < ubo.min.x || frag_pos.x > ubo.max.x ||
    frag_pos.y < ubo.min.y || frag_pos.y > ubo.max.y)
    discard;

    float radius = ubo.corner_radius;
    vec2 inner_min = ubo.min + vec2(radius);
    vec2 inner_max = ubo.max - vec2(radius);

    vec2 nearest = clamp(frag_pos, inner_min, inner_max);
    float dist = length(frag_pos - nearest);

    if (dist > radius)
    discard;

    frag_color = ubo.color;
}
