#version 460

layout (location = 0) out vec4 uFragColor;

layout(push_constant) uniform constants {
    mat4 view_proj;
    vec4 a;
    vec4 b;
    vec4 color;
} pc;

void main() {
    uFragColor = pc.color;
}
