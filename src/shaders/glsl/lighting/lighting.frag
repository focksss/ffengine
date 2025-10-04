#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

layout (location = 0) out vec4 uFragColor;

layout (location = 0) in vec2 uv;
layout(set = 0, binding = 0) uniform sampler2D g_material;
layout(set = 0, binding = 1) uniform sampler2D g_albedo;
layout(set = 0, binding = 2) uniform sampler2D g_metallic_roughness;
layout(set = 0, binding = 3) uniform sampler2D g_extra_properties;
layout(set = 0, binding = 4) uniform sampler2D g_depth;
layout(set = 0, binding = 5) uniform sampler2D g_view_normal;

layout(set = 0, binding = 6) uniform sampler2D shadowmap;

layout(binding = 7) uniform UniformBuffer {
    mat4 view;
    mat4 projection;
} ubo;

struct Light {
    mat4 projection;
    mat4 view;
    vec3 vector;
};

layout(set = 0, binding = 8, std430) readonly buffer LightsSSBO {
    Light lights[];
} lights_SSBO;

vec3 get_position_from_depth() {
    float z = texture(g_depth, uv).r;
    float x = uv.x * 2.0 - 1.0;
    float y = uv.y * 2.0 - 1.0;

    vec4 projected_position = vec4(x, y, z, 1.0);

    vec4 view_space_position = inverse(ubo.projection) * projected_position;

    return view_space_position.xyz / view_space_position.w;
}

float get_shadow(Light light, vec3 world_position, vec3 world_normal) {
    vec4 position_lightspace = light.projection * light.view * vec4(world_position, 1.0);
    vec3 projected_lightspace_position;
    projected_lightspace_position.xy = (position_lightspace.xy / position_lightspace.w) * 0.5 + 0.5;
    projected_lightspace_position.z = position_lightspace.z / position_lightspace.w;
    projected_lightspace_position.y = projected_lightspace_position.y;
    float current_depth = projected_lightspace_position.z;

    float closest_depth = texture(shadowmap, projected_lightspace_position.xy).r;

    vec3 light_direction = normalize(light.vector);
    float bias = max(0.0001 * (1.0 - dot(world_normal, -light_direction)), 0.0001);

    float shadow = 0.0;
    vec2 texel_size = 1.0 / textureSize(shadowmap, 0);
    for (int x = -1; x <= 1; ++x) {
        for (int y = -1; y <= 1; ++y) {
            float pcf_depth = texture(shadowmap, projected_lightspace_position.xy + vec2(x, y) * texel_size).r;
            shadow += current_depth + bias < pcf_depth  ? 1.0 : 0.0;
        }
    }
    shadow /= 9.0;

    //if (projected_lightspace_position.z > 1.0) shadow = 0.0;

    return 1.0 - shadow;
}

void main() {
    //uFragColor = vec4(0.01 / texture(g_depth, uv).r, 0.0, 0.0, 1.0);
    mat4 inverse_view = inverse(ubo.view);

    vec3 albedo = texture(g_albedo, uv).rgb;

    vec3 view_position = get_position_from_depth();
    vec3 world_position = (inverse_view * vec4(view_position, 1.0)).xyz * vec3(1.0, 1.0, 1.0);

    vec3 view_normal = (texture(g_view_normal, uv).xyz * 2.0) - 1.0;
    vec3 world_normal = mat3(inverse_view) * view_normal;

    //uFragColor = vec4(world_normal, 1.0);
    uFragColor = vec4(albedo * max(0.2, get_shadow(lights_SSBO.lights[0], world_position, world_normal) * max(0.0, dot(world_normal, -normalize(lights_SSBO.lights[0].vector)))), 1.0);
}
