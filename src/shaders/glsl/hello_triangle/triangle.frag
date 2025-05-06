#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

layout (location = 0) out vec4 uFragColor;

layout (location = 0) in vec3 fragPos;
layout (location = 1) in vec3 o_color;
layout (location = 2) in vec2 o_uv;
layout (location = 3 ) flat in uint material;

layout(set = 0, binding = 2) uniform sampler2D textures[];

struct Material {
    uint normal_tex;      // 0
    vec4 specular_color;  // 16
    float ior;            // 32
    float padding0;       // 36
    vec4 base_color;      // 48
    uint base_color_tex;  // 64
    float metallic;       // 68
    uint metallic_tex;    // 72
    float roughness;      // 76
    uint roughness_tex;   // 80
    float padding;        // 84
};

layout(set = 0, binding = 1, std430) readonly buffer MaterialSSBO {
    Material materials[];
} materialSSBO;

void main() {
    Material mat = materialSSBO.materials[material];
    uFragColor = vec4(texture(textures[mat.normal_tex], o_uv).rgb, 1.0);
}
