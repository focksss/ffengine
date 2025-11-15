#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

// layout (location = 0) out ivec4 frag_material;
layout (location = 0) out vec4 frag_albedo;
layout (location = 1) out vec4 frag_metallic_roughness;
layout (location = 2) out vec4 frag_extra_material_properties;
layout (location = 3) out vec4 frag_view_normal;

layout (location = 0) in vec3 o_view_normal;
layout (location = 1) in vec2 o_uv;
layout (location = 2) flat in uint material;
layout (location = 3) in mat3 view_TBN;

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
    Material mat = materialSSBO.materials[material];
    vec3 normal;
    vec3 view_normal;
    if (mat.normal_tex > -1) {
        vec2 transformed_uv = o_uv * mat.normal_tex_scale + mat.normal_tex_offset;
        vec3 mapped_normal = texture(textures[mat.normal_tex], transformed_uv).rgb;
        mapped_normal = normalize(mapped_normal * 2.0 - 1.0);
        view_normal = normalize(view_TBN * mapped_normal);
    } else {
        view_normal = o_view_normal;
    }

    vec3 emission;
    if (mat.emissive_texture > -1) {
        vec2 transformed_uv = o_uv * mat.emissive_tex_scale + mat.emissive_tex_offset;
        emission = texture(textures[mat.emissive_texture], transformed_uv).rgb;
    } else {
        emission = mat.emissive_color.rgb;
    }

    vec4 base_color;
    if (mat.base_color_tex > -1) {
        vec2 transformed_uv = o_uv * mat.base_color_tex_scale + mat.base_color_tex_offset;
        base_color = texture(textures[mat.base_color_tex], transformed_uv);
    } else {
        base_color = mat.base_color;
    }

    // frag_material = ivec4(material);
    frag_albedo = vec4(base_color);
    frag_metallic_roughness = vec4(mat.metallic, mat.roughness, 1.0, 1.0);
    frag_extra_material_properties = vec4(emission, mat.emissive_strength);
    frag_view_normal = vec4(view_normal * 0.5 + 0.5, 1.0); // convert normal to 0-1 scale

    if (base_color.a < mat.alpha_cutoff) {
        discard;
        frag_albedo.a = 0.0;
        frag_metallic_roughness.a = 0.0;
        frag_extra_material_properties.a = 0.0;
        frag_view_normal.a = 0.0;
    }
}
