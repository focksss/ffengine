#version 460

layout (location = 0) out vec4 uFragColor;

layout (location = 0) in vec2 uv;
layout(set = 0, binding = 0) uniform sampler2D lighting;
layout(set = 0, binding = 1) uniform sampler2D text;

void main() {
    vec4 lighting_color = texture(lighting, uv);
    vec4 text_color = texture(text, uv);
    uFragColor = mix(lighting_color, text_color, text_color.a);
}
