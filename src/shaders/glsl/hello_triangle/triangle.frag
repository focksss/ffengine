#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (location = 0) out vec4 uFragColor;

layout (location = 0) in vec3 fragPos;
layout (location = 1) in vec3 o_color;
layout (location = 2) in vec2 o_uv;
layout (location = 3 ) flat in uint material;

layout(set = 0, binding = 1) uniform sampler2D textures[];

layout(set = 0, binding = 2) buffer MaterialSSBO {
    vec4 materials[];
};

void main() {
    uFragColor = vec4(texture(textures[0],o_uv).rgb, 1.0);
}
