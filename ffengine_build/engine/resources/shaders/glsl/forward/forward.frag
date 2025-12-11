#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

// layout (location = 0) out ivec4 frag_material;
layout (location = 0) out vec4 color;

layout (location = 0) in vec3 o_view_normal;
layout (location = 1) in vec2 o_uv;
layout (location = 2) flat in uint material;
layout (location = 3) in mat3 view_TBN;
layout (location = 6) flat in uvec2 o_id;

layout(set = 0, binding = 2) uniform sampler2D textures[];

struct Material {
    int normal_tex;
    float alpha_cutoff;
    float emissive_strength;
    int emissive_texture;

    vec2 normal_tex_offset;
    vec2 normal_tex_scale;

    vec2 emissive_tex_offset;
    vec2 emissive_tex_scale;

    vec4 specular_color;

    vec4 emissive_color;

    float ior;
    float _pad3_0;
    float _pad3_1;
    float _pad3_2;

    vec4 base_color;

    int base_color_tex;
    float metallic;
    int metallic_tex;
    float roughness;

    int roughness_tex;
    float _pad4_0;
    float _pad4_1;
    float _pad4_2;

    vec2 base_color_tex_offset;
    vec2 base_color_tex_scale;

    vec2 metallic_tex_offset;
    vec2 metallic_tex_scale;

    vec2 roughness_tex_offset;
    vec2 roughness_tex_scale;
};

layout(set = 0, binding = 0, std430) readonly buffer MaterialSSBO {
    Material materials[];
} materialSSBO;

void main() {
    color = vec4(1.0, 0.0, 1.0, 1.0);
}
