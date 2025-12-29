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

layout(binding = 2) uniform ViewportUBO {
    vec4 data;
} viewport;

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

    mat3 view_normal_matrix = transpose(inverse(mat3(ubo.view * model_matrix)));
    vec3 view_normal = normalize(view_normal_matrix * normal);

    vec2 ndc_offset = normalize((ubo.projection * vec4(view_normal, 0.0)).xy);

    vec2 pixel_ndc = vec2(
        2.0 / viewport.data.x,
        2.0 / viewport.data.y
    );

    vec4 position = ubo.projection * ubo.view * model_matrix * vec4(pos, 1.0);
    position.xy += ndc_offset * pixel_ndc * 5.0 * position.w;

    gl_Position = position;
}
