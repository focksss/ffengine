#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

layout (location = 0) in vec3 direction;

layout (location = 0) out vec4 color;

const float PI = 3.14159263;

layout(set = 0, binding = 2) uniform sampler2D textures[];

void main() {
    gl_FragDepth = 0.0;

    vec3 dir = normalize(direction);
    float phi = atan(dir.z, dir.x); // azimuth angle
    float theta = asin(dir.y); // elevation angle

    vec2 uv;
    uv.x = phi / (2.0 * PI) + 0.5; // horizontal wrapping
    uv.y = 1.0 - (theta / PI + 0.5); // vertical mapping

    color = textureLod(textures[0], uv, 0.0);
}
