#version 460
#extension GL_EXT_multiview : enable

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

const mat4 proj = mat4(
    1, 0, 0, 0,
    0, -1, 0, 0,
    0, 0, 0, -1,
    0, 0, 0.01, 0
);
const mat4 views[6] = mat4[](
// +X
mat4(
0.0,  0.0, -1.0, 0.0,
0.0, -1.0,  0.0, 0.0,
-1.0,  0.0,  0.0, 0.0,
0.0,  0.0,  0.0, 1.0
),

// -X
mat4(
0.0,  0.0,  1.0, 0.0,
0.0, -1.0,  0.0, 0.0,
1.0,  0.0,  0.0, 0.0,
0.0,  0.0,  0.0, 1.0
),

// +Y
mat4(
1.0,  0.0,  0.0, 0.0,
0.0,  0.0,  1.0, 0.0,
0.0, -1.0,  0.0, 0.0,
0.0,  0.0,  0.0, 1.0
),

// -Y
mat4(
1.0,  0.0,  0.0, 0.0,
0.0,  0.0, -1.0, 0.0,
0.0,  1.0,  0.0, 0.0,
0.0,  0.0,  0.0, 1.0
),

// +Z
mat4(
1.0,  0.0,  0.0, 0.0,
0.0, -1.0,  0.0, 0.0,
0.0,  0.0, -1.0, 0.0,
0.0,  0.0,  0.0, 1.0
),

// -Z
mat4(
-1.0,  0.0,  0.0, 0.0,
0.0, -1.0,  0.0, 0.0,
0.0,  0.0,  1.0, 0.0,
0.0,  0.0,  0.0, 1.0
)
);

layout (location = 0) out vec3 local_position;

void main() {
    local_position = vertices[gl_ViewIndex];
    gl_Position = proj * views[gl_ViewIndex] * vec4(local_position, 1.0);
}
