#version 400
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (location = 0) out vec4 uFragColor;


layout (location = 0) in vec3 fragPos;
layout (location = 1) in vec4 o_color;
void main() {
    uFragColor = o_color;
}
