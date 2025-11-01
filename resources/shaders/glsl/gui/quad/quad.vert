#version 460

const vec2 vertices[6] = vec2[](
vec2(0.0, 0.0),
vec2(1.0, 0.0),
vec2(0.0, 1.0),
vec2(0.0, 1.0),
vec2(1.0, 0.0),
vec2(1.0, 1.0)
);

layout(push_constant) uniform constants {
    vec4 color;
    ivec2 resolution;
    vec2 min;
    vec2 max;
    vec2 position;
    vec2 scale;
} ubo;

layout (location = 0) out vec2 uv;
layout (location = 1) out vec2 o_pos;

void main() {
    uv = vertices[gl_VertexIndex];

    vec2 pos = ((ubo.position + ubo.scale * vec2(uv.x, 1.0 - uv.y)) / vec2(ubo.resolution));
    o_pos = pos;
    vec2 ndc = pos * 2.0 - 1.0;

    gl_Position = vec4(ndc * vec2(1, -1), 0.0, 1.0);
}