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

const float MIN = 10.0;
const float MAX = 20.0;
const float RANGE = 20.0;
const float SMOOTH = 150.0;
const int STEPS = 100;
const bool FOLLOW = false;
const float MINIMUM_DENSITY = 0.3;
const float DENSITY_MULTIPLIER = 1.0;
const float DETAIL_WEIGHT = 0.1;

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

float lerp(float a, float b, float t) {
    return a + t * (b - a);
}

float sample_density(vec3 p) {
    float weight = min(SMOOTH, min(p.y - MIN, MAX - p.y)) / SMOOTH;
    vec4 shape = texture(high, p * 0.00025);
    vec4 detail = texture(low, p * 0.05);
    float density = max(0.0, lerp(shape.x, detail.x, DETAIL_WEIGHT) - MINIMUM_DENSITY) * DENSITY_MULTIPLIER;
    return density;
}

float lightmarch(vec3 p, vec3 cloud_min, vec3 cloud_max) {

    vec3 d = normalize(vec3(1000.0));
    float t_min = 0.0;
    float t_max = 0.0;
    intersect(p, d, cloud_min, cloud_max, t_min, t_max);

    float distance_within = t_max - t_min;
    float step_size = distance_within / STEPS;
    vec3 sample_p = p + 0.5 * d * step_size;

    float accum_density = 0.0;
    for (int i = 0; i < STEPS; i++) {
        accum_density += max(0.0, sample_density(sample_p) * step_size);
        p += d * step_size;
    }

    float darkness_threshold = 0.2;
    return darkness_threshold * exp(-(accum_density * 1.0)) * (1.0 - darkness_threshold);
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

        float step_size = travel_distance / float(STEPS) ;
        float t = 0.0;

        //debug
        float accum_density = 0.0;
        while (t < travel_distance) {
            vec3 p = entrance + t * d;

            accum_density += step_size * max(0.0, texture(high, p * 0.025).r - 0.5) * 5.0;

            t += step_size;
        }
        color = vec4(1.0) * (1.0 - exp(-accum_density));

        /*
        float transmittance = 1.0;
        vec3 energy = vec3(0.0);
        while (t < travel_distance) {
            vec3 p = entrance + t * d;

            if (sample_depth(p) < geometry_depth) { break; }

            float density = sample_density(p);
            if (density > 0) {
                energy += density * step_size * transmittance * lightmarch(p, cloud_min, cloud_max);
                transmittance *= exp(-density * step_size);
            }

            if (transmittance < 0.01) { break; }

            t += step_size;
        }
        color = vec4(1.0) * vec4(energy, 1.0 - transmittance);
        */
    } else {
        discard;
    }
}
