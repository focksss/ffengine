#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

layout(triangles, invocations = 5) in;
layout(triangle_strip, max_vertices = 3) out;

layout(location = 0) in vec2 in_uv[];
layout(location = 1) flat in uint in_material[];

layout(location = 0) out vec2 o_uv;
layout(location = 1) flat out uint material;

layout(binding = 2) uniform SunUBO {
    mat4 matrices[5];
    vec3 vector;
} sun;

void main() {
    for (int i = 0; i < 3; ++i) {
        gl_Position = sun.matrices[gl_InvocationID] * gl_in[i].gl_Position;
        gl_Layer = gl_InvocationID;

        o_uv = in_uv[i];
        material = in_material[i];

        EmitVertex();
    }
    EndPrimitive();
}
