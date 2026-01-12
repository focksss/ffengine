#version 460

layout(set = 0, binding = 0) uniform sampler2D depth;
layout(set = 0, binding = 1) uniform sampler3D high;
layout(set = 0, binding = 2) uniform sampler3D low;

layout (location = 0) in vec2 uv;

layout (location = 0) out vec4 color;

layout(push_constant) uniform push_constants {
    mat4 view;
    mat4 projection;
} constants;

const float MIN = -20.0;
const float MAX = 400.0;
const float RANGE = 100000.0;
const int STEPS = 20;
const bool FOLLOW = false;

bool intersect(
    vec3 o,
    vec3 d,
    vec3 bmin,
    vec3 bmax,
    out float t_min,
    out float t_max
) {
    vec3 inv_d = 1.0 / d;

    vec3 t0 = (bmin - o) * inv_d;
    vec3 t1 = (bmax - o) * inv_d;

    vec3 t_m = min(t0, t1);
    vec3 t_M = max(t0, t1);

    t_min = max(0.01, max(max(t_m.x, t_m.y), t_m.z));
    t_max = min(min(t_M.x, t_M.y), t_M.z);

    return t_max >= t_min;
}

float sample_depth(vec3 p) {
    vec4 view_pos = constants.view * vec4(p, 1.0);
    vec4 proj_pos = constants.projection * view_pos;
    return proj_pos.z / proj_pos.w;
}

void main() {
    mat4 inverse_projection = inverse(constants.projection);
    mat4 inverse_view = inverse(constants.view);

    vec2 ndc = uv * 2.0 - 1.0;
    vec4 clip = vec4(ndc, 1.0, 1.0);
    vec4 view = inverse_projection * clip;
    view /= view.w;
    vec3 d = normalize((inverse_view * vec4(view.xyz, 0.0)).xyz);
    vec3 o = (inverse_view * vec4(0.0, 0.0, 0.0, 1.0)).xyz;

    vec3 offset = FOLLOW ? o : vec3(0.0);
    vec3 cloud_min = vec3(offset.x - RANGE, MIN, offset.z - RANGE);
    vec3 cloud_max = vec3(offset.x + RANGE, MAX, offset.z + RANGE);

    float t_min = 0.0;
    float t_max = 0.0;
    bool ray_hit = intersect(o, d, cloud_min, cloud_max, t_min, t_max);
    if (!ray_hit) { discard; }

    vec3 entrance = o + t_min * d;

    float entrance_depth = sample_depth(entrance);

    float geometry_depth = texture(depth, uv).r;

    bool hit_cloud = entrance_depth > geometry_depth;
    if (hit_cloud) {
        // color = texture(high, entrance * 0.05); return;
        float travel_distance = t_max - t_min;

        float step_size = travel_distance / float(STEPS);
        float t = 0.0;
        float accum_density = 0.0;
        while (t < travel_distance) {
            vec3 p = entrance + t * d;

            if (sample_depth(p) < geometry_depth) { break; }

            vec4 sampled = mix(texture(high, p * 0.0005), texture(low, p * 0.05), 0.1);
            float density = max(0.0, sampled.r - 0.4);
            accum_density += density;

            t += step_size;
        }
        float transmittance = 1.0 - exp(-accum_density);
        color = vec4(1.0) * transmittance;
    } else {
        discard;
    }
}
