#version 400
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (location = 0) in vec4 pos;
layout (location = 1) in vec4 color;

layout (location = 0) out vec3 fragPos;
layout (location = 1) out vec4 o_color;

layout(binding = 0) uniform UniformBufferObject {
    mat4 view;
} ubo;
void main() {
    fragPos = pos.xyz;
    o_color = color;
    gl_Position = pos;
}
