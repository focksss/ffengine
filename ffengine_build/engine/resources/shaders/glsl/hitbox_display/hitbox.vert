#version 460

layout(push_constant) uniform constants {
    mat4 view_proj;
    vec4 a;
    vec4 b;
    vec4 color;
} pc;

void main() {
    if (gl_VertexIndex == 0) {
        gl_Position = pc.view_proj * vec4(pc.a.xyz, 1.0);
    } else {
        gl_Position = pc.view_proj * vec4(pc.b.xyz, 1.0);
    }
}
