#version 460

layout(push_constant) uniform constants {
    mat4 view_proj;
    vec4 center;
    vec4 half_extent;
    vec4 quat;
} pc;

mat3 quat_to_mat3(vec4 q) {
    float xx = q.x * q.x;
    float yy = q.y * q.y;
    float zz = q.z * q.z;
    float xy = q.x * q.y;
    float xz = q.x * q.z;
    float yz = q.y * q.z;
    float wx = q.w * q.x;
    float wy = q.w * q.y;
    float wz = q.w * q.z;

    return mat3(
        1.0 - 2.0 * (yy + zz), 2.0 * (xy + wz), 2.0 * (xz - wy),
        2.0 * (xy - wz), 1.0 - 2.0 * (xx + zz), 2.0 * (yz + wx),
        2.0 * (xz + wy), 2.0 * (yz - wx), 1.0 - 2.0 * (xx + yy)
    );
}

void main() {
    const vec3 cube_verts[8] = vec3[8](
        vec3(-1, -1, -1), vec3(1, -1, -1),
        vec3( 1, 1, -1), vec3(-1, 1, -1),
        vec3(-1, -1, 1), vec3(1, -1, 1),
        vec3(1, 1, 1), vec3(-1, 1, 1)
    );

    const int indices[36] = int[36](
    0,1,2, 2,3,0,
    4,6,5, 6,4,7,
    0,4,5, 5,1,0,
    2,6,7, 7,3,2,
    0,3,7, 7,4,0,
    1,5,6, 6,2,1
    );

    vec3 local_pos = cube_verts[indices[gl_VertexIndex]];
    local_pos *= pc.half_extent.xyz;
    mat3 rotation = quat_to_mat3(pc.quat);
    vec3 rotated_pos = rotation * local_pos;
    vec3 world_pos = rotated_pos + pc.center.xyz;
    gl_Position = pc.view_proj * vec4(world_pos, 1.0);
}
