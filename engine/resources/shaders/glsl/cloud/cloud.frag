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

const float MIN = 50.0;
const float MAX = 150.0;
const float RANGE = 100.0;
const int STEPS = 8;
const bool FOLLOW = false;
const float SHAPING_SCALE = 0.00125;
const float DETAILING_SCALE = 0.05;
const float DENSITY_OFFSET = -0.5;
const float DENSITY_MULTIPLIER = 5.0;
const float DETAIL_WEIGHT = 0.3;

const vec3 SUN_DIR = vec3(-1.0);

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

float remap(float v, float min_old, float max_old, float min_new, float max_new) {
    return min_new + (v - min_old) * (max_new - min_new) / (max_old - min_old);
}
float saturate(float v) {
    return clamp(v, 0.0, 1.0);
}

float sample_density(vec3 p) {
    float sample_height = (p.y - MIN) / (MAX - MIN);
    //https://www.desmos.com/calculator/rqxctltcfe
    float gradient = smoothstep(0.0, 0.2, sample_height) * smoothstep(1.0, 0.3, sample_height);
    vec4 shaping_sample = texture(high, p * SHAPING_SCALE);
    vec4 shape_weights = shaping_sample / dot(shaping_sample, vec4(1.0));
    float shaping = dot(shaping_sample, shape_weights) * gradient;
    float shape_density = shaping + DENSITY_OFFSET;

    if (shape_density > 0.0) {
        vec4 detailing_sample = texture(low, p * DETAILING_SCALE);
        vec3 detailing_weights = detailing_sample.xyz / dot(detailing_sample.xyz, vec3(1.0));
        float detailing = dot(detailing_sample.xyz, detailing_weights);

        float erosion = (1.0 - shaping) * (1.0 - shaping) * (1.0 - shaping);
        float density = shape_density - (1.0 - detailing) * erosion * DETAIL_WEIGHT;
        return density * DENSITY_MULTIPLIER;
    }
    return 0;
}

float henyey_greenstein(float a, float g) {
    float g2 = g*g;
    return (1.0 - g2) / (4.0 * 3.1415 * pow(1.0 + g2 - 2.0 * g * a, 1.5));
}
float phase(float a) {
    // forward scatter, back scatter, base brightness, phase factor
    vec4 factors = vec4(0.83, 0.3, 0.8, 0.15);

    float blend = 0.5;
    float henyey_greenstein_blend = henyey_greenstein(a, factors.x) * (1.0 - blend) + henyey_greenstein(a, -factors.y) * blend;
    return factors.z + henyey_greenstein_blend * factors.w;
}

float lightmarch(vec3 p, vec3 cloud_min, vec3 cloud_max) {

    vec3 d = normalize(SUN_DIR);
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
    return darkness_threshold + exp(-accum_density) * (1.0 - darkness_threshold);
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
    bool do_march = intersect(o, d, cloud_min, cloud_max, t_min, t_max);

    float geometry_depth = texture(depth, uv).r;
    vec4 geometry_ndc = vec4(ndc, geometry_depth, 1.0);
    vec4 view_pos = inverse_projection * geometry_ndc;
    view_pos /= view_pos.w;
    vec3 geometry_p = (inverse_view * vec4(view_pos.xyz, 1.0)).xyz;
    float t_geometry = dot(geometry_p - o, d);

    if (t_min > t_geometry) do_march = false;

    float cos_theta = dot(d, -normalize(SUN_DIR));
    float phase = phase(cos_theta);

    float travel_distance = min(t_max, t_geometry) - t_min;

    float transmittance = 1.0;
    vec3 energy = vec3(0.0);
    if (do_march) {
        vec3 entrance = o + t_min * d;
        float step_size = 0.5;
        float t = 0.0;
        while (t < travel_distance) {
            vec3 p = entrance + t * d;

            float density = sample_density(p);
            if (density > 0.0) {

                float sample_energy = lightmarch(p, cloud_min, cloud_max);
                energy += density * step_size * transmittance * sample_energy * phase;
                transmittance *= exp(-density * step_size);

                if (transmittance < 0.01) break;
            }

            t += step_size;
            step_size *= 1.01;
        }
    }

    float fog = 1.0 - exp(-max(0.0, t_geometry) * 0.001);

    vec3 cloud_col = energy;
    vec3 sky_fog = vec3(0.2, 0.2, 0.8) * fog;

    float alpha = (1.0 - fog) * transmittance;

    vec3 overlay = sky_fog * transmittance + cloud_col;

    float eye_focus = pow(saturate(cos_theta), 1.3);
    float sun = saturate(henyey_greenstein(eye_focus, 0.9995)) * transmittance;

    overlay = overlay * (1.0 - sun) + vec3(0.95, 0.95, 0.8) * sun;

    color = vec4(overlay, 1.0 - alpha);
}
