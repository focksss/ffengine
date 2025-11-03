#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

const float PI = 3.141592;

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

layout(push_constant) uniform push_constants {
    mat4 inverse_view;
    mat4 inverse_projection;
} constants;

struct Light {
    vec3 position;
    vec3 direction;
    uint type; // 0 = point, 1 = directional, 2 = spotlight
    vec3 falloffs; // x = quadratic, y = linear, z = constant. For spotlights, x = inner cutoff, y = cutoff
    vec3 color;
};

layout(set = 0, binding = 8, std430) readonly buffer LightsSSBO {
    Light lights[];
} lights_SSBO;

layout(binding = 10) uniform SunUBO {
    mat4 matrices[5];
    vec3 vector;
    vec3 color;
} sun;

layout(binding = 9) uniform UniformBuffer {
    vec4 cascade_plane_distances;
    int num_lights;
} ubo;

vec3 get_position_from_depth() {
    float z = texture(g_depth, uv).r;
    float x = uv.x * 2.0 - 1.0;
    float y = uv.y * 2.0 - 1.0;

    vec4 projected_position = vec4(x, y, z, 1.0);

    vec4 view_space_position = constants.inverse_projection * projected_position;

    return view_space_position.xyz / view_space_position.w;
}

float attenuation(vec3 l_pos, vec3 pos, float constant, float linear, float quadratic) {
    float distance = length(l_pos - pos);
    return 1 / (constant + linear*distance + quadratic*(distance*distance));
}
vec4 get_lighting(Light l, vec3 pos) {
    vec4 ret;
    if (l.type == 0) {
        ret.w = attenuation(l.position, pos, l.falloffs.z, l.falloffs.y, l.falloffs.x);
        ret.xyz = normalize(l.position - pos);
    } else if (l.type == 1) {
        ret.w = 1;
        ret.xyz = -normalize(l.direction);
    } else if (l.type == 2) {
        float theta = dot(normalize(pos - l.position), normalize(l.direction));
        float epsilon = l.falloffs.y - l.falloffs.x;
        float intensity = clamp((l.falloffs.x - theta) / epsilon, 0, 1);
        if (theta > l.falloffs.y) {
            ret.w = intensity;
            ret.xyz = normalize(l.position - pos);
        }
    }
    return ret;
}

float get_shadow(vec3 world_position, vec3 world_normal, float fragment_depth) {
    vec4 res = step(ubo.cascade_plane_distances, vec4(fragment_depth));
    int layer = int(res.x + res.y + res.z + res.w);

    vec4 position_lightspace = sun.matrices[layer] * vec4(world_position, 1.0);
    vec3 projected_lightspace_position;
    projected_lightspace_position.xy = (position_lightspace.xy / position_lightspace.w) * 0.5 + 0.5;
    projected_lightspace_position.z = position_lightspace.z / position_lightspace.w;
    projected_lightspace_position.y = projected_lightspace_position.y;
    float current_depth = projected_lightspace_position.z;

    float closest_depth = texture(shadowmap, vec3(projected_lightspace_position.xy, layer)).r;

    vec3 light_direction = normalize(sun.vector);
    float bias = max(0.05 * (1.0 - dot(world_normal, -light_direction)), 0.005) / (ubo.cascade_plane_distances[layer]);

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


vec3 fresnel_schlick(float cos_theta, vec3 F0) {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}
vec3 fresnel_schlick_roughness(float cos_theta, vec3 F0, float roughness) {
    return F0 + (max(vec3(1.0 - roughness), F0) - F0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}
float distribution_GGX(vec3 N, vec3 H, float roughness) {
    float a = roughness*roughness;
    float a2 = a*a;
    float NdotH = max(dot(N, H), 0.0);
    float NdotH2 = NdotH*NdotH;

    float num = a2;
    float denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;

    return num / denom;
}
float geometry_schlick_GGX(float NdotV, float roughness) {
    float r = (roughness + 1.0);
    float k = (r*r) / 8.0;

    float num = NdotV;
    float denom = NdotV * (1.0 - k) + k;

    return num / denom;
}
float geometry_smith(vec3 N, vec3 V, vec3 L, float roughness) {
    float NdotV = max(dot(N, V), 0.0);
    float NdotL = max(dot(N, L), 0.0);
    float ggx2 = geometry_schlick_GGX(NdotV, roughness);
    float ggx1 = geometry_schlick_GGX(NdotL, roughness);

    return ggx1 * ggx2;
}


void main() {
    vec4 sampled_albedo = texture(g_albedo, uv);
    if (sampled_albedo.a == 0) { uFragColor = vec4(0.0); return; }
    vec3 albedo = sampled_albedo.rgb;
    // uFragColor = vec4(vec3(texture(ssao_tex, uv).r), 1.0); return;

    //uFragColor = vec4(0.01 / texture(g_depth, uv).r, 0.0, 0.0, 1.0);

    mat4 inverse_view = constants.inverse_view;

    vec3 view_position = get_position_from_depth();
    vec3 world_position = (inverse_view * vec4(view_position, 1.0)).xyz * vec3(1.0, 1.0, 1.0);

    vec3 view_normal = (texture(g_view_normal, uv).xyz * 2.0) - 1.0;
    vec3 N = mat3(inverse_view) * view_normal;
    vec3 V = normalize(-view_position);
    vec3 R = reflect(-V, N);

    float ao = texture(ssao_tex, uv).r;

    vec4 extra_properties = texture(g_extra_properties, uv);
    vec3 emission_color = extra_properties.rgb;
    float emission_strength = extra_properties.a;

    vec4 metallic_roughness = texture(g_metallic_roughness, uv);
    float metallic = metallic_roughness.r;
    float roughness = metallic_roughness.g;

    // uFragColor = vec4(albedo, 1.0); return;

    float cos_theta = max(dot(N, V), 0.001);

    vec3 F0 = vec3(0.04);
    F0 = mix(F0, albedo, metallic);
    vec3 k_s = fresnel_schlick(cos_theta, F0);
    vec3 k_d = 1.0 - k_s;
    k_d *= 1.0 - metallic;
    vec3 irradiance = vec3(1.0);
    vec3 diffuse = irradiance * albedo;

    // vec3 F = fresnel_schlick_roughness(cos_theta, F0, roughness);

    // vec3 ambient = (k_d * diffuse) * ao + emission_color * emission_strength;
    vec3 ambient = albedo * ao + emission_strength * emission_color;

    vec3 frag_radiance = ambient;

    // sunlight
    {
        vec3 Wi = -sun.vector;

        float cos_theta_light = max(dot(N, Wi), 0.001);

        vec3 H = normalize(V + Wi);
        float cos_theta_halfway = max(dot(H, V), 0.001);
        vec3 F = fresnel_schlick_roughness(cos_theta_halfway, F0, roughness);
        float NDF = distribution_GGX(N, H, roughness);
        float G = geometry_smith(N, V, Wi, roughness);
        vec3 numerator = NDF * G * F;
        float denominator = 4.0 * cos_theta * cos_theta_light;
        vec3 specular = numerator / denominator;
        vec3 k_d_light = (1.0 - F) * (1.0 - metallic);
        frag_radiance += (k_d_light * albedo / PI + specular) * sun.color * cos_theta_light * get_shadow(world_position, N, -view_position.z);
    }
    for (int i = 0; i < ubo.num_lights; i++) {
        Light l = lights_SSBO.lights[i];

        vec4 lighting_info = get_lighting(l, world_position);
        float atten = lighting_info.w;
        if (atten < 0.001) continue;
        vec3 Wi = lighting_info.xyz;
        float cos_theta_light = max(dot(N, Wi), 0.001);
        vec3 radiance = l.color * atten;

        vec3 H = normalize(V + Wi);
        float cos_theta_halfway = max(dot(H, V), 0.001);
        vec3 F = fresnel_schlick_roughness(cos_theta_halfway, F0, roughness);
        float NDF = distribution_GGX(N, H, roughness);
        float G = geometry_smith(N, V, Wi, roughness);
        vec3 numerator = NDF * G * F;
        float denominator = 4.0 * cos_theta * cos_theta_light;
        vec3 specular = numerator / denominator;
        vec3 k_d_light = (1.0 - F) * (1.0 - metallic);

        frag_radiance += (k_d_light * albedo / PI + specular) * radiance * cos_theta_light;
    }

    float gamma = 1.0;
    float exposure = 1.0;
    uFragColor = vec4(pow(vec3(1.0) - exp(-frag_radiance.rgb * exposure), vec3(1.0 / gamma)).rgb, 1.0);
}
