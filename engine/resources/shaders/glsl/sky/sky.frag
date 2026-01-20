#version 460

layout(set = 0, binding = 0) uniform sampler2D depth;
layout(set = 0, binding = 1) uniform sampler3D cloud_shaping;
layout(set = 0, binding = 2) uniform sampler3D cloud_detailing;
layout(set = 0, binding = 3) uniform sampler2D cloud_weather_map;
layout(set = 0, binding = 4) uniform sampler2D atmosphere_transmittance_lut;
layout(set = 0, binding = 5) uniform sampler2D atmosphere_sky_view_lut;
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
const float PLANET_RADIUS = 6360e3;
const float PLANET_RADIUS_MM = 6.36;
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

const float PI = 3.14159263;

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

vec3 jodie_reinhard_tonemap(vec3 c){
    float l = dot(c, vec3(0.2126, 0.7152, 0.0722));
    vec3 tc = c / (c + 1.0);
    return mix(c / (l + 1.0), tc, tc);
}
vec3 get_transmittance(vec3 p, vec3 sun_dir) {
    vec3 o_MM = p * 1e-6 + vec3(0.0, PLANET_RADIUS_MM + 0.0002, 0.0);
    o_MM.y = max(PLANET_RADIUS_MM, o_MM.y);
    float height = length(o_MM);
    vec3 up = o_MM / height;
    float sun_cos_zenith_angle = dot(sun_dir, up);
    vec2 uv = vec2(
    clamp(0.5 + 0.5 * sun_cos_zenith_angle, 0.0, 1.0),
    max(0.0, min(1.0, (height - PLANET_RADIUS) / (ATMOSPHERE_RADIUS - PLANET_RADIUS)))
    );
    return texture(atmosphere_transmittance_lut, uv).rgb;
}
vec3 calculate_atmosphere(vec3 o, vec3 d) {
    vec3 o_MM = o * 1e-6 + vec3(0.0, PLANET_RADIUS_MM + 0.0002, 0.0);
    o_MM.y = max(PLANET_RADIUS_MM, o_MM.y);
    float height = length(o_MM);
    vec3 up = o_MM / height;

    float horizon = acos(clamp(sqrt(height * height - 6.360 * 6.360) / height, -1.0, 1.0));
    float altitude = horizon - acos(dot(d, up));

    float azimuth;
    if (abs(altitude) > (0.5 * PI - .0001)) {
        azimuth = 0.0;
    } else {
        vec3 right = cross(SUN_DIR, up);
        vec3 forward = cross(up, right);

        vec3 projected_d = normalize(d - up * (dot(d, up)));
        float sin_theta = dot(projected_d, right);
        float cos_theta = dot(projected_d, forward);
        azimuth = atan(sin_theta, cos_theta) + PI;
    }

    float v = 0.5 + 0.5 * sign(altitude) * sqrt(abs(altitude) * 2.0 / PI);
    vec2 sky_uv = vec2(azimuth / (2.0 * PI), v);

    vec3 luminance = texture(atmosphere_sky_view_lut, sky_uv).rgb;

    // /*

    // Tonemapping and gamma. Super ad-hoc, probably a better way to do this.
    luminance *= 20.0;
    luminance = pow(luminance, vec3(1.3));
    luminance /= (smoothstep(0.0, 0.2, clamp(SUN_DIR.y, 0.0, 1.0))*2.0 + 0.15);

    luminance = jodie_reinhard_tonemap(luminance);

    luminance = pow(luminance, vec3(1.0/2.2));
    // */

    return luminance;
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

    vec3 d = SUN_DIR;
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

    return shadow * calculate_atmosphere(p, SUN_DIR);
}

void main() {
    //
    //if (uv.x < 0.5) {
    //color = texture(atmosphere_transmittance_lut, uv * vec2(2.0, 1.0));
    //} else {
    //color = vec4(texture(atmosphere_multiscatter_lut, uv * vec2(2.0, 1.0)).rgb, 1.0);
    //}return;


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

    float max_atmosphere_dist = sky_hit ? 1e6 : t_geometry;

    if (t_min > t_geometry) do_march = false;

    float cos_theta = dot(d, SUN_DIR);
    float phase = phase(cos_theta);

    float travel_distance = min(t_max, t_geometry) - t_min;

    float cloud_transmittance = 1.0;
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
                energy += density * step_size * cloud_transmittance * sample_energy * phase;
                cloud_transmittance *= exp(-density * step_size * ABSORPTION_THROUGH_CLOUD);

                if (cloud_transmittance < 0.01) break;
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

        vec3 sun_transmittance = get_transmittance(o * 1e-6 + vec3(0.0, PLANET_RADIUS + 0.0002, 0.0), SUN_DIR);
        sun = sun_col * sun_factor * sun_transmittance * cloud_transmittance;
    }

    vec3 atmosphere_color = calculate_atmosphere(o, d);

    vec3 atmosphere_transmittance = get_transmittance(o, SUN_DIR);

    vec3 final_color = atmosphere_color;

    if (do_march) {
        final_color = final_color * cloud_transmittance + energy;
    }

    final_color += sun;

    if (!sky_hit) {
        final_color *= atmosphere_transmittance;
    }

    float final_alpha = sky_hit ? 1.0 : (1.0 - cloud_transmittance * atmosphere_transmittance.r);

    color = vec4(final_color, final_alpha);
}