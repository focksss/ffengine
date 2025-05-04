#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (set = 0, location = 0) in vec3 pos;
layout (set = 0, location = 1) in vec3 color;
layout (set = 0, location = 2) in vec2 uv;

layout (location = 3) in mat4 model;
layout (location = 7) in uint material;

layout (location = 0) out vec3 fragPos;
layout (location = 1) out vec3 o_color;
layout (location = 2) out vec2 o_uv;

layout(binding = 0) uniform UniformBuffer {
    mat4 view;
    mat4 projection;
} ubo;
void main() {
    fragPos = pos;
    o_color = color;
    o_uv = vec2(uv.x, 1 - uv.y);
    gl_Position = ubo.projection * ubo.view * model * vec4(pos, 1.0);
}
