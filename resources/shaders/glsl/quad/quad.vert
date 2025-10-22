#version 460

const vec2 vertices[6] = vec2[](
vec2(-1.0, -1.0), // Bottom-left
vec2( 1.0, -1.0), // Bottom-right
vec2(-1.0,  1.0), // Top-left
vec2(-1.0,  1.0), // Top-left
vec2( 1.0, -1.0), // Bottom-right
vec2( 1.0,  1.0)  // Top-right
);

layout (location = 0) out vec2 uv;

void main() {
    uv = (vertices[gl_VertexIndex] + 1.0) * 0.5;
    gl_Position = vec4(vertices[gl_VertexIndex], 0, 1.0);
}