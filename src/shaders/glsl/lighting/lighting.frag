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

layout(set = 0, binding = 6) uniform sampler2DArray shadowmap;
layout(set = 0, binding = 7) uniform sampler2D ssao_tex;

layout(push_constant) uniform constants {
    mat4 view;
    mat4 projection;
} ubo;

struct Light {
    mat4 projections[5];
    mat4 views[5];
    vec3 vector;
};

layout(set = 0, binding = 8, std430) readonly buffer LightsSSBO {
    Light lights[];
} lights_SSBO;

layout(binding = 9) uniform UniformBuffer {
    vec4 cascade_plane_distances;
} ubo2;

vec3 get_position_from_depth() {
    float z = texture(g_depth, uv).r;
    float x = uv.x * 2.0 - 1.0;
    float y = uv.y * 2.0 - 1.0;

    vec4 projected_position = vec4(x, y, z, 1.0);

    vec4 view_space_position = inverse(ubo.projection) * projected_position;

    return view_space_position.xyz / view_space_position.w;
}

float get_shadow(Light light, vec3 world_position, vec3 world_normal, float fragment_depth) {
    int layer = int(fragment_depth >= ubo2.cascade_plane_distances.x)
    + int(fragment_depth >= ubo2.cascade_plane_distances.y)
    + int(fragment_depth >= ubo2.cascade_plane_distances.z)
    + int(fragment_depth >= ubo2.cascade_plane_distances.w);

    vec4 position_lightspace = light.projections[layer] * light.views[layer] * vec4(world_position, 1.0);
    vec3 projected_lightspace_position;
    projected_lightspace_position.xy = (position_lightspace.xy / position_lightspace.w) * 0.5 + 0.5;
    projected_lightspace_position.z = position_lightspace.z / position_lightspace.w;
    projected_lightspace_position.y = projected_lightspace_position.y;
    float current_depth = projected_lightspace_position.z;

    float closest_depth = texture(shadowmap, vec3(projected_lightspace_position.xy, layer)).r;

    vec3 light_direction = normalize(light.vector);
    float bias = max(0.0001 * (1.0 - dot(world_normal, -light_direction)), 0.0001);

    float shadow = 0.0;
    vec2 texel_size = 1.0 / textureSize(shadowmap, 0).xy;
    for (int x = -1; x <= 1; ++x) {
        for (int y = -1; y <= 1; ++y) {
            float pcf_depth = texture(shadowmap, vec3(projected_lightspace_position.xy + vec2(x, y) * texel_size, layer)).r;
            shadow += current_depth + bias < pcf_depth  ? 1.0 : 0.0;
        }
    }
    shadow /= 9.0;

    //if (projected_lightspace_position.z > 1.0) shadow = 0.0;

    return 1.0 - shadow;
}

void main() {
    //uFragColor = vec4(vec3(texture(ssao_tex, uv).r), 1.0); return;
    //uFragColor = vec4(0.01 / texture(g_depth, uv).r, 0.0, 0.0, 1.0);
    if (uv.y < 0.2) {
        if (uv.x < 0.2) {
            uFragColor = vec4(vec3(texture(shadowmap, vec3(uv * 5.0, 0)).r), 1.0);
            return;
        } else if (uv.x < 0.4) {
            uFragColor = vec4(vec3(texture(shadowmap, vec3((uv - vec2(0.2, 0.0)) * 5.0, 1)).r), 1.0);
            return;
        } else if (uv.x < 0.6) {
            uFragColor = vec4(vec3(texture(shadowmap, vec3((uv - vec2(0.4, 0.0)) * 5.0, 2)).r), 1.0);
            return;
        } else if (uv.x < 0.8) {
            uFragColor = vec4(vec3(texture(shadowmap, vec3((uv - vec2(0.6, 0.0)) * 5.0, 3)).r), 1.0);
            return;
        } else if (uv.x < 1.0) {
            uFragColor = vec4(vec3(texture(shadowmap, vec3((uv - vec2(0.8, 0.0)) * 5.0, 4)).r), 1.0);
            return;
        }
    }

    mat4 inverse_view = inverse(ubo.view);

    vec3 albedo = texture(g_albedo, uv).rgb;

    vec3 view_position = get_position_from_depth();
    vec3 world_position = (inverse_view * vec4(view_position, 1.0)).xyz * vec3(1.0, 1.0, 1.0);

    vec3 view_normal = (texture(g_view_normal, uv).xyz * 2.0) - 1.0;
    vec3 world_normal = mat3(inverse_view) * view_normal;

    int layer = int(-view_position.z >= ubo2.cascade_plane_distances.x)
    + int(-view_position.z >= ubo2.cascade_plane_distances.y)
    + int(-view_position.z >= ubo2.cascade_plane_distances.z)
    + int(-view_position.z >= ubo2.cascade_plane_distances.w);
    if (layer == 0) {
        uFragColor = vec4(vec3(1.0, 0.0, 0.0), 1.0);
    } else if (layer == 1) {
        uFragColor = vec4(vec3(0.0, 1.0, 0.0), 1.0);
    } else if (layer == 2) {
        uFragColor = vec4(vec3(0.0, 0.0, 1.0), 1.0);
    } else if (layer == 3) {
        uFragColor = vec4(vec3(1.0, 0.0, 1.0), 1.0);
    } else if (layer == 4) {
        uFragColor = vec4(vec3(1.0, 0.5, 0.0), 1.0);
    }
    uFragColor.xyz *= max(0.2, get_shadow(lights_SSBO.lights[0], world_position, world_normal, -view_position.z)) * texture(ssao_tex, uv).r;
    return;

    uFragColor = vec4(
        albedo
        * texture(ssao_tex, uv).r
        * max(0.2, get_shadow(lights_SSBO.lights[0], world_position, world_normal, -view_position.z)
        * max(0.0, dot(world_normal, -normalize(lights_SSBO.lights[0].vector))))
    , 1.0);
}
