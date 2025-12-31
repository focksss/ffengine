#version 460

layout (location = 0) in vec3 pos;

layout(push_constant) uniform constants {
    mat4 mvp;
    mat4 spare;
} pc;

void main() {
    gl_Position = pc.mvp * vec4(pos, 1.0);
}
