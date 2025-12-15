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

layout(push_constant) uniform camera_constants {
    mat4 view;
    mat4 projection;
} constants;

layout (location = 0) out vec3 direction;

void main() {
    vec3 local_direction = vertices[gl_VertexIndex];

    mat3 view_rotation = inverse(mat3(constants.view));

    direction = view_rotation * local_direction;

    gl_Position = constants.projection * vec4(local_direction, 1.0);
}