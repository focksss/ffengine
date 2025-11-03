#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

layout (location = 0) out ivec4 frag_material;
layout (location = 1) out vec4 frag_albedo;
layout (location = 2) out vec4 frag_metallic_roughness;
layout (location = 3) out vec4 frag_extra_material_properties;
layout (location = 4) out vec4 frag_view_normal;

layout (location = 0) in vec3 o_view_normal;
layout (location = 1) in vec2 o_uv;
layout (location = 2) flat in uint material;
layout (location = 3) in mat3 view_TBN;

layout(set = 0, binding = 2) uniform sampler2D textures[];

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
    vec3 normal;
    vec3 view_normal;
    if (mat.normal_tex > -1) {
        vec3 mapped_normal = texture(textures[mat.normal_tex], o_uv).rgb;
        mapped_normal = normalize(mapped_normal * 2.0 - 1.0);
        view_normal = normalize(view_TBN * mapped_normal);
    } else {
        view_normal = o_view_normal;
    }

    vec4 base_color;
    if (mat.base_color_tex > -1) {
        base_color = texture(textures[mat.base_color_tex], o_uv);
    } else {
        base_color = mat.base_color;
    }
    if (base_color.a < mat.alpha_cutoff) {
        discard;
    }

    vec3 emission;
    if (mat.emissive_texture > -1) {
        emission = texture(textures[mat.emissive_texture], o_uv).rgb;
    } else {
        emission = mat.emissive_color.rgb;
    }

    frag_material = ivec4(material);
    frag_albedo = vec4(base_color);
    frag_metallic_roughness = vec4(mat.metallic, mat.roughness, 1.0, 1.0);
    frag_extra_material_properties = vec4(emission, mat.emissive_strength);
    frag_view_normal = vec4(view_normal * 0.5 + 0.5, 1.0); // convert normal to 0-1 scale
}
