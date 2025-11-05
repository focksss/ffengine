#version 460

layout (location = 0) out vec4 uFragColor;

layout (location = 0) in vec2 uv;
layout(set = 0, binding = 0) uniform sampler2D lighting;
layout(set = 0, binding = 1) uniform sampler2D hitbox;
layout(set = 0, binding = 2) uniform sampler2D gui;

void main() {
    vec4 lighting_color = texture(lighting, uv);
    vec4 hitbox_color = texture(hitbox, uv);
    vec4 gui_color = texture(gui, uv);

    uFragColor = mix(lighting_color, hitbox_color, hitbox_color.a);
    uFragColor = mix(uFragColor, gui_color, gui_color.a);
}
