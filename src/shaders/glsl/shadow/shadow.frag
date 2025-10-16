#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

layout (location = 0) in vec2 o_uv;
layout (location = 1) flat in uint material;

layout(set = 0, binding = 3) uniform sampler2D textures[];

struct Material {
    int normal_tex;      // 0
    float alpha_cutoff;
    float emissive_strength;
    int emissive_texture;
    vec4 specular_color;
    vec4 emissive_color;
    float ior;            // 32
    vec4 base_color;      // 48
    int base_color_tex;  // 64
    float metallic;       // 68
    int metallic_tex;    // 72
    float roughness;      // 76
    int roughness_tex;   // 80
};

layout(set = 0, binding = 0, std430) readonly buffer MaterialSSBO {
    Material materials[];
} materialSSBO;

void main() {
    Material mat = materialSSBO.materials[material];
    vec4 base_color;
    if (mat.base_color_tex > -1) {
        base_color = texture(textures[mat.base_color_tex], o_uv);
    } else {
        base_color = mat.base_color;
    }
    if (base_color.a < 1.0) {
        discard;
    }
}
