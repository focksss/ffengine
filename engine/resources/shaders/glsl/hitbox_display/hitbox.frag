#version 460

layout (location = 0) out vec4 uFragColor;

layout(push_constant) uniform constants {
    mat4 mvp;
    mat4 spare;
} pc;

void main() {
    uFragColor = pc.spare[0];
}
