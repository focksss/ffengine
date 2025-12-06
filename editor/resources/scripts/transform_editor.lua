local gui

function Awake()
    gui = Engine.renderer:gui(0)
end

local time_since_text_update = 0
function update_translation_x_display()
	time_since_text_update = time_since_text_update + dt

	if time_since_text_update > 0.1 then
        gui.ActiveNode:get_text_at(0):update_text(string.format("%.3f", Engine.scene:get_transform(selected_transform).translation.x))
        time_since_text_update = 0
        frame_count = 0
	end
end

function color_slider_hovered()
    gui.ActiveNode:get_quad_at(0).color = Vector.new4(0.4, 0.4, 0.4, 1.0)
end

function color_slider_unhovered()
    gui.ActiveNode:get_quad_at(0).color = Vector.new4(0.35, 0.35, 0.35, 1.0)
end

local original_cursor_x
local original_translation
function begin_drag()
    original_cursor_x = Engine.client.cursor_position.x
    original_translation = Engine.scene:get_transform(selected_transform).translation
end

function update_drag()
    local transform = Engine.scene:get_transform(selected_transform)
    local delta = Engine.client.cursor_position.x - original_cursor_x
    transform.translation = Vector.new3(
        original_translation.x + delta * 0.005,
        transform.translation.y,
        transform.translation.z
    )
end