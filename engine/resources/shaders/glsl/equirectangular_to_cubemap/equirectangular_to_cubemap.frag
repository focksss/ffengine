#version 460

layout (location = 0) in vec3 local_position;

layout (location = 0) out vec4 color;

layout(set = 0, binding = 0) uniform sampler2D equirectangular_map;

layout(push_constant) uniform push_constants {
    mat4 view;
    int call_index;
} constants;

const vec2 inv_atan = vec2(0.1591, 0.3183);
vec2 sample_spherical(vec3 v)
{
    vec2 uv = vec2(atan(v.z, v.x), asin(v.y));
    uv *= inv_atan;
    uv += 0.5;
    return uv;
}

void main() {
    vec2 uv = sample_spherical(normalize(local_position));
    color = texture(equirectangular_map, vec2(uv.x, 1.0 - uv.y));
}
