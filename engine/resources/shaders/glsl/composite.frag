#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

layout (location = 0) out vec4 uFragColor;

layout (location = 0) in vec2 uv;
layout(set = 0, binding = 0) uniform sampler2D textures[];

layout(push_constant) uniform constants {
    int count;
} ubo;

void main() {
    uFragColor = vec4(0.0);
    for (int i = 0; i < ubo.count; i++) {
        vec4 sampled = texture(textures[i], uv);
        uFragColor = mix(uFragColor, sampled, sampled.a);
    }
}
