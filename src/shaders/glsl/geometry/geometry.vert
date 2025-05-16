#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (location = 0) in vec3 pos;
layout (location = 1) in vec3 normal;
layout (location = 2) in vec2 uv;
layout (location = 3) in vec3 tangent;
layout (location = 4) in vec3 bitangent;
layout (location = 5) in uvec4 joint_indices;
layout (location = 6) in vec4 weights;

layout (location = 7) in mat4 model;
layout (location = 11) in uint material;

layout (location = 0) out vec3 fragPos;
layout (location = 1) out vec3 o_normal;
layout (location = 2) out vec2 o_uv;
layout (location = 3) out uint o_material;
layout (location = 4) out mat3 TBN;
layout (location = 7) out mat3 viewTBN;

layout(binding = 0) uniform UniformBuffer {
    mat4 view;
    mat4 projection;
} ubo;

layout(set = 0, binding = 2, std430) readonly buffer JointsSSBO {
    mat4 joint_matrices[];
} jointsSSBO;

void main() {
    mat4 model_matrix = model;
    if (joint_indices.x+joint_indices.y+joint_indices.z+joint_indices.w != 0) {
        model_matrix =
            weights.x * jointsSSBO.joint_matrices[joint_indices.x] +
            weights.y * jointsSSBO.joint_matrices[joint_indices.y] +
            weights.z * jointsSSBO.joint_matrices[joint_indices.z] +
            weights.w * jointsSSBO.joint_matrices[joint_indices.w];
    }

    vec4 position = model_matrix * vec4(pos, 1.0);
    fragPos = position.xyz;
    o_uv = vec2(uv.x, uv.y);
    o_material = material;
    gl_Position = ubo.projection * ubo.view * position;

    mat3 normalMatrix = mat3(transpose(inverse(model_matrix)));
    mat3 viewNormalMatrix = transpose(inverse(mat3(ubo.view * model_matrix)));
    o_normal = normalize(normalMatrix * normal);
    vec3 vertexViewNormal = normalize(viewNormalMatrix * normal);

    vec3 T = normalize(normalMatrix * tangent);
    vec3 B = normalize(normalMatrix * bitangent);
    TBN = mat3(T, B, o_normal);
    vec3 viewT = normalize(viewNormalMatrix * tangent);
    vec3 viewB = normalize(viewNormalMatrix * bitangent);
    viewTBN = mat3(viewT, viewB, vertexViewNormal);
}
