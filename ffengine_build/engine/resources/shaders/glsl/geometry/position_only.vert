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
layout (location = 11) in ivec4 indices;

layout(push_constant) uniform constants {
    mat4 view;
    mat4 projection;
} ubo;

layout(set = 0, binding = 1, std430) readonly buffer JointsSSBO {
    mat4 joint_matrices[];
} joints_SSBO;

void main() {
    int skin = indices.y;
    mat4 model_matrix = model;
    if (skin > -1) {
        uint joint_offset = uint((joints_SSBO.joint_matrices[uint(skin)])[0][0]);
        model_matrix =
        weights.x * joints_SSBO.joint_matrices[joint_indices.x + joint_offset] +
        weights.y * joints_SSBO.joint_matrices[joint_indices.y + joint_offset] +
        weights.z * joints_SSBO.joint_matrices[joint_indices.z + joint_offset] +
        weights.w * joints_SSBO.joint_matrices[joint_indices.w + joint_offset];
    }

    vec4 position = model_matrix * vec4(pos, 1.0);
    gl_Position = ubo.projection * ubo.view * position;
}
