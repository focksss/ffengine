#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (location = 0) in vec3 pos;
layout (location = 1) in vec3 normal;
layout (location = 2) in vec2 uv;
layout (location = 3) in vec3 tangent;
layout (location = 4) in vec3 bitangent;

layout (location = 5) in mat4 model;
layout (location = 9) in uint material;

layout (location = 0) out vec3 fragPos;
layout (location = 1) out vec3 o_normal;
layout (location = 2) out vec2 o_uv;
layout (location = 3) out uint o_material;
layout (location = 4) out mat3 TBN;
//layout (location = 5) out mat3 viewTBN;

layout(binding = 0) uniform UniformBuffer {
    mat4 view;
    mat4 projection;
} ubo;

void main() {
    fragPos = pos;
    o_uv = vec2(uv.x, uv.y);
    o_material = material;
    gl_Position = ubo.projection * ubo.view * model * vec4(pos, 1.0);

    mat3 normalMatrix = mat3(transpose(inverse(model)));
//    mat3 viewNormalMatrix = transpose(inverse(mat3(ubo.view * model)));
    o_normal = normalize(normalMatrix * normal);
//    vertexViewNormal = normalize(viewNormalMatrix * normal);

    vec3 T = normalize(normalMatrix * tangent);
    vec3 B = normalize(normalMatrix * bitangent);
    TBN = mat3(T, B, o_normal);
//    vec3 viewT = normalize(viewNormalMatrix * aTangent);
//    vec3 viewB = normalize(viewNormalMatrix * aBitangent);
//    viewTBN = mat3(viewT, viewB, vertexViewNormal);
}
