#version 460
// per MM, as per: https://www.shadertoy.com/view/slSXRW
const float PLANET_RADIUS = 6.360;
const float ATMOSPHERE_RADIUS = 6.460;
const vec3 RAYLEIGH_SCATTERING = vec3(5.802, 13.558, 33.1);
const float RAYLEIGH_ABSORPTION = 0.0;
const float MIE_SCATTERING = 21.996;
const float MIE_ABSORPTION = 4.4;
const vec3 OZONE_ABSORPTION = vec3(0.650, 1.881, .085);



layout (location = 0) in vec2 uv;

layout (location = 0) out vec4 color;

layout(set = 0, binding = 0) uniform sampler2D transmittance_lut;
layout(set = 0, binding = 1) uniform sampler2D multiscatter_lut;

layout(push_constant) uniform push_constants {
    vec3 SUN_DIRECTION;
    float pad;
    vec3 VIEW_POSITION;
};

const float PI = 3.14159263;

const int STEPS = 32;

float intersect_sphere(vec3 o, vec3 d, float r) {
    float b = dot(o, d);
    float c = dot(o, o) - r*r;
    if (c > 0.0f && b > 0.0) return -1.0;
    float discr = b*b - c;
    if (discr < 0.0) return -1.0;
    if (discr > b*b) return (-b + sqrt(discr));
    return -b - sqrt(discr);
}
void get_scattering_values(
vec3 p,
out vec3 rayleigh_scattering,
out float mie_scattering,
out vec3 extinction
) {
    float altitude = (length(p) - PLANET_RADIUS) * 1000.0; // km

    float rayleigh_density = exp(-altitude / 8.0);
    float mie_density = exp(-altitude / 1.2);

    rayleigh_scattering = RAYLEIGH_SCATTERING * rayleigh_density;
    float rayleigh_absorption = RAYLEIGH_ABSORPTION * rayleigh_density;

    mie_scattering = MIE_SCATTERING * mie_density;
    float mie_absorption = MIE_ABSORPTION * mie_density;

    vec3 ozone_absorption = OZONE_ABSORPTION * max(0.0, 1.0 - abs(altitude - 25.0) / 15.0);

    extinction = rayleigh_scattering + rayleigh_absorption + mie_scattering + mie_absorption + ozone_absorption;
}
vec3 get_transmittance(vec3 p, vec3 sun_dir) {
    float height = length(p);
    vec3 up = p / height;
    float sun_cos_zenith_angle = dot(sun_dir, up);
    vec2 uv = vec2(
    clamp(0.5 + 0.5 * sun_cos_zenith_angle, 0.0, 1.0),
    max(0.0, min(1.0, (height - PLANET_RADIUS) / (ATMOSPHERE_RADIUS - PLANET_RADIUS)))
    );
    return texture(transmittance_lut, uv).rgb;
}

float mie_phase(float cos_theta) {
    const float g = 0.8;
    const float scale = 3.0 / (8.0 * PI);

    float g2 = g*g;
    float numerator = (1.0 - g2) * (1.0 + cos_theta * cos_theta);
    float denominator = (2.0 + g2) * pow((1.0 + g2 - 2.0 * g * cos_theta), 1.5);

    return scale * numerator / denominator;
}
float rayleigh_phase(float cos_theta) {
    float k = 3.0 / (16.0 * PI);
    return k * (1.0 + cos_theta * cos_theta);
}
vec3 get_multiscatter(vec3 p, vec3 sun_dir) {
    float height = length(p);
    vec3 up = p / height;
    float sun_cos_zenith_angle = dot(sun_dir, up);
    vec2 sample_uv = vec2(
    clamp(0.5 + 0.5 * sun_cos_zenith_angle, 0.0, 1.0),
    max(0.0, min(1.0, (height - PLANET_RADIUS) / (ATMOSPHERE_RADIUS - PLANET_RADIUS)))
    );
    return texture(multiscatter_lut, sample_uv).rgb;
}

vec3 raymarch(
    vec3 o,
    vec3 d,
    vec3 sun_dir,
    float t_max,
    float steps
) {
    float cos_theta = dot(d, sun_dir);

    float mie_phase_value = mie_phase(cos_theta);
    float rayleigh_phase_value = rayleigh_phase(-cos_theta);

    vec3 luminance = vec3(0.0);
    vec3 transmittance = vec3(1.0);
    float t = 0.0;
    for (float i = 0.0; i < steps; i += 1.0) {
        float t_new = ((i + 0.3) / steps) * t_max;
        float dt = t_new - t;
        t = t_new;

        vec3 p = o + t * d;

        vec3 rayleigh_scattering;
        vec3 extinction;
        float mie_scattering;
        get_scattering_values(p, rayleigh_scattering, mie_scattering, extinction);

        vec3 sample_transmittance = exp(-dt * extinction);

        vec3 sun_transmittance = get_transmittance(p, sun_dir);
        vec3 psi_multiscatter = get_multiscatter(p, sun_dir);

        vec3 rayleigh_in_scattering = rayleigh_scattering * (rayleigh_phase_value * sun_transmittance + psi_multiscatter);
        vec3 mie_in_scattering = mie_scattering * (mie_phase_value * sun_transmittance + psi_multiscatter);
        vec3 in_scattering = (rayleigh_in_scattering + mie_in_scattering);

        vec3 scattering_integral = (in_scattering - in_scattering * sample_transmittance) / extinction;

        luminance += scattering_integral * transmittance;

        transmittance *= sample_transmittance;
    }
    return luminance;
}

void main() {
    float adj_v;
    if (uv.y < 0.5) {
        float coord = 1.0 - 2.0 * uv.y;
        adj_v = -coord * coord;
    } else {
        float coord = uv.y * 2.0 - 1.0;
        adj_v = coord * coord;
    }
    float azimuth = (uv.x - 0.5) * 2.0 * PI;

    vec3 view_pos = VIEW_POSITION * 1e-6 + vec3(0.0, PLANET_RADIUS + 0.0002, 0.0);
    view_pos.y = max(0.0001, view_pos.y);


    float height = length(view_pos);
    vec3 up = view_pos / height;
    float horizon = acos(clamp(sqrt(height * height - PLANET_RADIUS * PLANET_RADIUS) / height, -1.0, 1.0)) - 0.5 * PI;
    float altitude = adj_v * 0.5 * PI - horizon;

    float cos_altitude = cos(altitude);
    vec3 d = vec3(cos_altitude * sin(azimuth), sin(altitude), -cos_altitude * cos(azimuth));

    float sun_altitude = (0.5 * PI) - acos(dot(SUN_DIRECTION, up));
    vec3 sun_dir = vec3(0.0, sin(sun_altitude), -cos(sun_altitude));

    float t_atmosphere = intersect_sphere(view_pos, d, ATMOSPHERE_RADIUS);
    float t_ground = intersect_sphere(view_pos, d, PLANET_RADIUS);
    float t_max = (t_ground < 0.0) ? t_atmosphere : t_ground;

    vec3 luminance = raymarch(view_pos, d, sun_dir, t_max, float(STEPS));
    color = vec4(luminance, 1.0);
}