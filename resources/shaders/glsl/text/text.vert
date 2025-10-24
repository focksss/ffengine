#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (location = 0) in vec2 pos;
layout (location = 1) in vec2 uv;
layout (location = 2) in vec4 color;

layout (location = 0) out vec2 o_uv;
layout (location = 1) out vec4 o_color;

layout(push_constant) uniform constants {
    vec2 min;
    vec2 max;
    vec2 position;
    ivec2 resolution;
    int glyph_size;
    float distance_range;
} ubo;

void main() {
    float aspect_ratio = float(ubo.resolution.x) / float(ubo.resolution.y);
    o_uv = uv;
    o_color = color;
    gl_Position = vec4(vec2(ubo.position.x, -ubo.position.y) * 2.0 - 1.0 + (2.0 * vec2(pos.x / aspect_ratio, (-pos.y + 1.0))), 0.0, 1.0);
}
