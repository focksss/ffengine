#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

layout (location = 0) out vec4 uFragColor;

layout (location = 0) in vec2 uv;
layout(set = 0, binding = 0) uniform sampler2D g_material;
layout(set = 0, binding = 1) uniform sampler2D g_albedo;
layout(set = 0, binding = 2) uniform sampler2D g_metallic_roughness;
layout(set = 0, binding = 3) uniform sampler2D g_depth;
layout(set = 0, binding = 4) uniform sampler2D g_view_normal;

void main() {
    uFragColor = texture(g_depth, uv);
}
