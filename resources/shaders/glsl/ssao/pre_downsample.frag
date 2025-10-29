#version 460

layout(location = 0) out float depth_out;
layout(location = 1) out vec3 normal_out;

layout(location = 0) in vec2 uv;

layout(set = 0, binding = 0) uniform sampler2D depth_in;
layout(set = 0, binding = 1) uniform sampler2D normal_in;

void main() {
    ivec2 tex_size = textureSize(depth_in, 0) / 2;
    vec2 texel = 1.0 / vec2(tex_size);
    vec2 base_uv = uv * 2.0;

    vec3 best_normal = vec3(0.0);
    float best_depth = 1e9;

    for (int y = 0; y < 2; y++)
        for (int x = 0; x < 2; x++) {
            vec2 offset = vec2(x, y) * texel;
            float d = texture(depth_in, base_uv + offset).r;
            if (d < best_depth) {
                best_depth = d;
                best_normal = texture(normal_in, base_uv + offset).xyz;
            }
        }

    normal_out = normalize(best_normal);
    depth_out = best_depth;
}
