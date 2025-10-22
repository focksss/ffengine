#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (location = 0) in vec3 pos;
layout (location = 2) in vec2 uv;
layout (location = 5) in uvec4 joint_indices;
layout (location = 6) in vec4 weights;

layout (location = 7) in mat4 model;
layout (location = 11) in ivec2 indices;

layout (location = 0) out vec2 o_uv;
layout (location = 1) out uint o_material;

layout(set = 0, binding = 1, std430) readonly buffer JointsSSBO {
    mat4 joint_matrices[];
} joints_SSBO;

void main() {
    int material = indices.x;
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

    gl_Position = model_matrix * vec4(pos, 1.0);

    o_uv = vec2(uv.x, uv.y);
    o_material = material;
}
