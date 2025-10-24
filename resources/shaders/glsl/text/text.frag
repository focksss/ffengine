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
} ubo;

void main() {
    frag_color = vec4(texture(atlas, uv).rgb, 1.0);
}
