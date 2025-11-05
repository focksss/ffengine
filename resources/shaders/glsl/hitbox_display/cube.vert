#version 460

layout(push_constant) uniform constants {
    mat4 view_proj;
    vec4 min;
    vec4 max;
} pc;

const uint indices[36] = uint[36](
0,1,2, 2,1,3,
4,6,5, 5,6,7,
0,2,4, 4,2,6,
1,5,3, 3,5,7,
0,4,1, 1,4,5,
2,3,6, 6,3,7
);

void main() {
    uint vid = indices[gl_VertexIndex];

    vec3 corner = vec3(
    (vid >> 0u) & 1u,
    (vid >> 1u) & 1u,
    (vid >> 2u) & 1u
    );

    vec3 pos = mix(pc.min.xyz, pc.max.xyz, corner);

    gl_Position = pc.view_proj * vec4(pos, 1.0);
}
