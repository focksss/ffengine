#version 460

layout(set = 0, binding = 0) uniform sampler2D depth;
layout(set = 0, binding = 1) uniform sampler3D cloud_shaping;
layout(set = 0, binding = 2) uniform sampler3D cloud_detailing;
layout(set = 0, binding = 3) uniform sampler2D cloud_weather_map;
layout(set = 0, binding = 4) uniform sampler2D atmosphere_transmittance_lut;
layout(set = 0, binding = 5) uniform sampler2D atmosphere_multiscatter_lut;
layout(set = 0, binding = 6) uniform SunUBO {
    vec3 SUN_DIR;
    float pad;
    float time;
};

layout (location = 0) in vec2 uv;

layout (location = 0) out vec4 color;

layout(push_constant) uniform push_constants {
    mat4 view;
    mat4 projection;
} constants;

const float MIN = 150.0;
const float MAX = 450.0;
const float RANGE = 3000.0;
const int STEPS = 9;
const bool FOLLOW = true;
const float SHAPING_SCALE = 0.0003;
const float DETAILING_SCALE = 0.003;
const float DENSITY_OFFSET = -0.2;
const float DENSITY_MULTIPLIER = 1.0;
const float DETAIL_WEIGHT = 4.5;
const float DARKNESS_THRESHOLD = 0.5;
// each component influences decreasing scales
const vec4 SHAPE_WEIGHTS = vec4(6.1, 1.64, 3.18, 4.24);
const vec3 DETAIL_WEIGHTS = vec3(1.5, 2.0, 1.5);

// proportional to "powder sugar" effect
const float ABSORPTION_TOWARDS_SUN = 0.7;
// proportional to raincloud effect
const float ABSORPTION_THROUGH_CLOUD = 1.1;

const vec3 WIND = -5.0 * vec3(0.002, 0.0, 0.005);
const float SHAPE_SPEED = 1.0;
const float DETAIL_SPEED = -1.0;

// atmosphere constants
const float PLANET_RADIUS = 6371e3;
const float ATMOSPHERE_RADIUS = 6471e3;
const vec3 PLANET_CENTER = vec3(0.0, -PLANET_RADIUS, 0.0);

// air molecule scattering, stronger at blue wavelengths
const vec3 RAYLEIGH_SCATTERING = vec3(5.8e-6, 13.5e-6, 33.1e-6);
const float RAYLEIGH_SCALE_HEIGHT = 8000.0;
// particulate scattering, wavelength independent
const vec3 MIE_SCATTERING = vec3(21e-6);
const float MIE_SCALE_HEIGHT = 1200.0;
const float MIE_G = 0.76; // anisotropy
const vec3 SUN_INTENSITY = vec3(20.0);
const int ATMOSPHERE_IN_SCATTERING_STEPS = 8;
const int ATMOSPHERE_OPTICAL_DEPTH_STEPS = 8;

bool intersect_sphere(
    vec3 o,
    vec3 d,
    vec3 center,
    float radius,
    out float t0,
    out float t1
) {
    vec3 oc = o - center;
    float b = dot(oc, d);
    float c = dot(oc, oc) - radius * radius;
    float discriminant = b * b - c;

    if (discriminant < 0.0) return false;

    float sqrt_disc = sqrt(discriminant);
    t0 = -b - sqrt_disc;
    t1 = -b + sqrt_disc;

    return t1 >= 0.0;
}

bool intersect_aabb(
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

vec2 atmosphere_density(vec3 p) {
    float height = length(p - PLANET_CENTER) - PLANET_RADIUS;
    height = max(0.0, height);

    float rayleigh = exp(-height / RAYLEIGH_SCALE_HEIGHT);
    float mie = exp(-height / MIE_SCALE_HEIGHT);

    return vec2(rayleigh, mie);
}
float rayleigh_phase(float cos_theta) {
    return (3.0 / (16.0 * 3.14159)) * (1.0 + cos_theta * cos_theta);
}
float mie_phase(float cos_theta, float g) {
    float g2 = g * g;
    return (1.0 / (4.0 * 3.14159)) * ((1.0 - g2) / pow(1.0 + g2 - 2.0 * g * cos_theta, 1.5));
}
// returns rayleigh-mie
vec2 optical_depth(vec3 o, vec3 d, float t) {
    vec3 p = o;
    float step_size = t / float(ATMOSPHERE_OPTICAL_DEPTH_STEPS);

    vec2 optical_depth_accum = vec2(0.0);

    for (int i = 0; i < ATMOSPHERE_OPTICAL_DEPTH_STEPS; i++) {
        vec2 density = atmosphere_density(p);
        optical_depth_accum += density * step_size;
        p += d * step_size;
    }

    return optical_depth_accum;
}
vec3 calculate_atmosphere(vec3 o, vec3 d, float max_dist) {
    float t_min, t_max;
    if (!intersect_sphere(o, d, PLANET_CENTER, ATMOSPHERE_RADIUS, t_min, t_max)) {
        return vec3(0.0);
    }

    // hit surface
    float t_ground_min, t_ground_max;
    bool hit_ground = intersect_sphere(o, d, PLANET_CENTER, PLANET_RADIUS, t_ground_min, t_ground_max);

    t_min = max(0.0, t_min);
    t_max = min(t_max, max_dist);

    if (hit_ground && t_ground_min > 0.0) {
        t_max = min(t_max, t_ground_min);
    }

    float t = t_max - t_min;
    if (t <= 0.0) return vec3(0.0);

    float step_size = t / float(ATMOSPHERE_IN_SCATTERING_STEPS);
    vec3 p = o + d * (t_min + step_size * 0.5);

    vec3 rayleigh_accum = vec3(0.0);
    vec3 mie_accum = vec3(0.0);
    vec2 optical_depth_accum = vec2(0.0);

    float cos_theta = dot(d, -SUN_DIR);
    float rayleigh_phase_value = rayleigh_phase(cos_theta);
    float mie_phase_value = mie_phase(cos_theta, MIE_G);

    for (int i = 0; i < ATMOSPHERE_IN_SCATTERING_STEPS; i++) {
        vec2 density = atmosphere_density(p);
        optical_depth_accum += density * step_size;

        // sun to sample point
        float t_light_min, t_light_max;
        intersect_sphere(p, -SUN_DIR, PLANET_CENTER, ATMOSPHERE_RADIUS, t_light_min, t_light_max);
        vec2 light_od = optical_depth(p, -SUN_DIR, t_light_max);

        // Calculate attenuation
        vec2 total_od = optical_depth_accum + light_od;
        vec3 attenuation = exp(-(RAYLEIGH_SCATTERING * total_od.x + MIE_SCATTERING * total_od.y));

        rayleigh_accum += density.x * attenuation * step_size;
        mie_accum += density.y * attenuation * step_size;

        p += d * step_size;
    }

    vec3 rayleigh = rayleigh_accum * RAYLEIGH_SCATTERING * rayleigh_phase_value;
    vec3 mie = mie_accum * MIE_SCATTERING * mie_phase_value;

    return (rayleigh + mie) * SUN_INTENSITY;
}
vec3 atmosphere_transmittance(vec3 o, vec3 d, float t) {
    float t_min, t_max;
    if (!intersect_sphere(o, d, PLANET_CENTER, ATMOSPHERE_RADIUS, t_min, t_max)) {
        return vec3(1.0);
    }

    t_min = max(0.0, t_min);
    t_max = min(t_max, t);

    vec2 od = optical_depth(o, d, t_max - t_min);
    return exp(-(RAYLEIGH_SCATTERING * od.x + MIE_SCATTERING * od.y));
}

float sample_density(vec3 p) {
    float sample_height = (p.y - MIN) / (MAX - MIN);
    // https://www.desmos.com/calculator/rqxctltcfe
    float gradient = smoothstep(0.0, 0.2, sample_height) * smoothstep(1.0, 0.3, sample_height);
    vec4 shaping_sample = texture(cloud_shaping, p * SHAPING_SCALE + time * WIND * SHAPE_SPEED);
    vec4 shape_weights = SHAPE_WEIGHTS / dot(SHAPE_WEIGHTS, vec4(1.0));
    float shaping = dot(shaping_sample, shape_weights) * gradient;
    float shape_density = shaping + DENSITY_OFFSET;

    if (shape_density > 0.0) {
        vec4 detailing_sample = texture(cloud_detailing, p * DETAILING_SCALE + time * WIND * DETAIL_SPEED);
        vec3 detailing_weights = DETAIL_WEIGHTS / dot(DETAIL_WEIGHTS, vec3(1.0));
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
    vec4 factors = vec4(0.83, 0.4, 0.8, 0.15);

    float blend = 0.5;
    float henyey_greenstein_blend = henyey_greenstein(a, factors.x) * (1.0 - blend) + henyey_greenstein(a, -factors.y) * blend;
    return factors.z + henyey_greenstein_blend * factors.w;
}
vec3 lightmarch(vec3 p, vec3 cloud_min, vec3 cloud_max) {

    vec3 d = -SUN_DIR;
    float t_min = 0.0;
    float t_max = 0.0;
    intersect_aabb(p, d, cloud_min, cloud_max, t_min, t_max);

    float distance_within = t_max - t_min;
    float step_size = distance_within / STEPS;
    vec3 sample_p = p + 0.5 * d * step_size;

    float accum_density = 0.0;
    for (int i = 0; i < STEPS; i++) {
        accum_density += max(0.0, sample_density(sample_p) * step_size);
        sample_p += d * step_size;
    }

    float shadow = DARKNESS_THRESHOLD + exp(-accum_density * ABSORPTION_TOWARDS_SUN) * (1.0 - DARKNESS_THRESHOLD);

    return shadow * calculate_atmosphere(p, -SUN_DIR, 10000.0);
}

void main() {
    /*
if (uv.x < 0.5) {
color = texture(atmosphere_transmittance_lut, uv * vec2(2.0, 1.0));
} else {
color = texture(atmosphere_multiscatter_lut, uv * vec2(2.0, 1.0));
}return;
*/

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
    bool do_march = intersect_aabb(o, d, cloud_min, cloud_max, t_min, t_max);

    float geometry_depth = texture(depth, uv).r;
    bool sky_hit = (geometry_depth == 0.0);
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
        float step_size = (MAX - MIN) * 0.01;
        float t = 0.0;
        while (t < travel_distance) {
            vec3 p = entrance + t * d;

            float density = sample_density(p);
            if (density > 0.0) {

                vec3 sample_energy = lightmarch(p, cloud_min, cloud_max);
                energy += density * step_size * transmittance * sample_energy * phase;
                transmittance *= exp(-density * step_size * ABSORPTION_THROUGH_CLOUD);

                if (transmittance < 0.01) break;
            }

            t += step_size;
            step_size *= 1.01;
        }
    }

    vec3 sun = vec3(0.0);
    if (sky_hit) {
        vec3 sun_col = 5.0 * vec3(0.95, 0.95, 0.8);
        float eye_focus = pow(saturate(cos_theta), 1.3);
        float sun_factor = saturate(henyey_greenstein(eye_focus, 0.9995));

        vec3 sun_transmittance = atmosphere_transmittance(o, d, 1e6);
        sun = sun_col * sun_factor * sun_transmittance * transmittance;
    }

    float max_atmosphere_dist = sky_hit ? 1e6 : t_geometry;
    vec3 atmosphere_color = calculate_atmosphere(o, d, max_atmosphere_dist);

    vec3 atmosphere_transmittance = atmosphere_transmittance(o, d, t_geometry);

    vec3 final_color = atmosphere_color;

    if (do_march) {
        final_color = final_color * transmittance + energy;
    }

    final_color += sun;

    if (!sky_hit) {
        final_color *= atmosphere_transmittance;
    }

    float final_alpha = sky_hit ? 1.0 : (1.0 - transmittance * atmosphere_transmittance.r);

    color = vec4(final_color, final_alpha);
}
