#version 460

vec3[] vertices = {
    // back face
    vec3(-1.0f, -1.0f, -1.0f),
    vec3( 1.0f,  1.0f, -1.0f),
    vec3( 1.0f, -1.0f, -1.0f),
    vec3( 1.0f,  1.0f, -1.0f),
    vec3(-1.0f, -1.0f, -1.0f),
    vec3(-1.0f,  1.0f, -1.0f),

    // front face
    vec3(-1.0f, -1.0f,  1.0f),
    vec3( 1.0f, -1.0f,  1.0f),
    vec3( 1.0f,  1.0f,  1.0f),
    vec3( 1.0f,  1.0f,  1.0f),
    vec3(-1.0f,  1.0f,  1.0f),
    vec3(-1.0f, -1.0f,  1.0f),

    // left face
    vec3(-1.0f,  1.0f,  1.0f),
    vec3(-1.0f,  1.0f, -1.0f),
    vec3(-1.0f, -1.0f, -1.0f),
    vec3(-1.0f, -1.0f, -1.0f),
    vec3(-1.0f, -1.0f,  1.0f),
    vec3(-1.0f,  1.0f,  1.0f),

    // right face
    vec3( 1.0f,  1.0f,  1.0f),
    vec3( 1.0f, -1.0f, -1.0f),
    vec3( 1.0f,  1.0f, -1.0f),
    vec3( 1.0f, -1.0f, -1.0f),
    vec3( 1.0f,  1.0f,  1.0f),
    vec3( 1.0f, -1.0f,  1.0f),

    // bottom face
    vec3(-1.0f, -1.0f, -1.0f),
    vec3( 1.0f, -1.0f, -1.0f),
    vec3( 1.0f, -1.0f,  1.0f),
    vec3( 1.0f, -1.0f,  1.0f),
    vec3(-1.0f, -1.0f,  1.0f),
    vec3(-1.0f, -1.0f, -1.0f),

    // top face
    vec3(-1.0f,  1.0f, -1.0f),
    vec3( 1.0f,  1.0f,  1.0f),
    vec3( 1.0f,  1.0f, -1.0f),
    vec3( 1.0f,  1.0f,  1.0f),
    vec3(-1.0f,  1.0f, -1.0f),
    vec3(-1.0f,  1.0f,  1.0f),
};

layout (location = 0) out vec3 local_position;

layout(push_constant) uniform push_constants {
    mat4 view;
    mat4 projection;
} constants;

void main() {
    local_position = vertices[gl_VertexIndex];
    gl_Position = constants.projection * constants.view * vec4(local_position, 1.0);
}
