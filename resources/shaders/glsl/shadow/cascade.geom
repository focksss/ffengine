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

layout(push_constant) uniform constants {
    int light_index;
} ubo;

struct Light {
    mat4 projections[5];
    mat4 views[5];
    vec3 vector;
};

layout(set = 0, binding = 2, std430) readonly buffer LightsSSBO {
    Light lights[];
} lights_SSBO;

void main() {
    for (int i = 0; i < 3; ++i) {
        gl_Position = lights_SSBO.lights[ubo.light_index].projections[gl_InvocationID] * lights_SSBO.lights[ubo.light_index].views[gl_InvocationID] * gl_in[i].gl_Position;
        gl_Layer = gl_InvocationID;

        o_uv = in_uv[i];
        material = in_material[i];

        EmitVertex();
    }
    EndPrimitive();
}
