#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

layout (location = 0) out vec4 uFragColor;

layout (location = 0) in vec3 fragPos;
layout (location = 1) in vec3 o_normal;
layout (location = 2) in vec2 o_uv;
layout (location = 3) flat in uint material;
layout (location = 4) in mat3 TBN;
//layout (location = 5) in mat3 viewTBN;

layout(set = 0, binding = 2) uniform sampler2D textures[];

struct Material {
    int normal_tex;      // 0
    vec4 specular_color;  // 16
    float ior;            // 32
    vec4 base_color;      // 48
    int base_color_tex;  // 64
    float metallic;       // 68
    int metallic_tex;    // 72
    float roughness;      // 76
    int roughness_tex;   // 80
};

layout(set = 0, binding = 1, std430) readonly buffer MaterialSSBO {
    Material materials[];
} materialSSBO;

void main() {
    Material mat = materialSSBO.materials[material];
    vec3 normal;
    if (mat.normal_tex > -1) {
        vec3 mapped_normal = texture(textures[mat.normal_tex], o_uv).rgb;
        mapped_normal = normalize(mapped_normal * 2 - 1);
        vec3 mapped_world_normal = normalize(TBN * mapped_normal);
    } else {
        normal = o_normal;
    }
    vec4 base_color;
    if (mat.base_color_tex > -1) {
        base_color = texture(textures[mat.base_color_tex], o_uv);
    } else {
        base_color = mat.base_color;
    }
    if (base_color.a < 0.5) {
        discard;
    }
    //uFragColor = vec4(base_color.rgb * max(0.2, dot(normal, normalize(vec3(1.0,1.0,1.0)))), 1.0);
    //uFragColor = vec4(base_color.rgb, 1.0);
    vec3 mapped = vec3(1.0) - exp(-base_color.rgb * 1.0);
    mapped = pow(mapped, vec3(1.0 / 1.8));
    uFragColor = vec4(mapped, 1.0);
}
