#version 460

layout(set = 0, binding = 0) uniform usampler2D stencil_buffer;

layout (location = 0) in vec2 uv;

layout (location = 0) out vec4 outline_color;

layout(push_constant) uniform constants {
    vec4 color;
    float thickness;
} pc;

void main() {
    uint center_stencil = texture(stencil_buffer, uv).r;

    if (center_stencil == 0u) {
        vec2 texel_size = 1.0 / vec2(textureSize(stencil_buffer, 0));

        bool is_edge = false;

        const int num_samples = 32;
        float max_distance = pc.thickness;

        for (int i = 0; i < num_samples; i++) {
            float angle = (float(i) / float(num_samples)) * 6.28318530718;
            vec2 offset = vec2(cos(angle), sin(angle)) * max_distance * texel_size;

            vec2 sample_uv = uv + offset;

            if (sample_uv.x < 0.0 || sample_uv.x > 1.0 ||
            sample_uv.y < 0.0 || sample_uv.y > 1.0) {
                continue;
            }

            if (texture(stencil_buffer, sample_uv).r == 1u) {
                is_edge = true;
                break;
            }
        }

        if (is_edge) {
            outline_color = pc.color;
        } else {
            discard;
        }
    } else {
        discard;
    }
}