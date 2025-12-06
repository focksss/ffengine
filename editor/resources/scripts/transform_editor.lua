local gui

function Awake()
    gui = Engine.renderer:gui(0)
end

local time_since_text_update = 0
function update_translation_x_display()
	time_since_text_update = time_since_text_update + dt

	if time_since_text_update > 0.1 then
        gui.ActiveNode:get_text_at(0):update_text(string.format("%.3f", Engine.scene:get_transform(_G.selected_transform).translation.x))
        time_since_text_update = 0
        frame_count = 0
	end
end
function update_translation_y_display()
	time_since_text_update = time_since_text_update + dt

	if time_since_text_update > 0.1 then
        gui.ActiveNode:get_text_at(0):update_text(string.format("%.3f", Engine.scene:get_transform(_G.selected_transform).translation.y))
        time_since_text_update = 0
        frame_count = 0
	end
end
function update_translation_z_display()
	time_since_text_update = time_since_text_update + dt

	if time_since_text_update > 0.1 then
        gui.ActiveNode:get_text_at(0):update_text(string.format("%.3f", Engine.scene:get_transform(_G.selected_transform).translation.z))
        time_since_text_update = 0
        frame_count = 0
	end
end
function update_rotation_x_display()
	time_since_text_update = time_since_text_update + dt

	if time_since_text_update > 0.1 then
        gui.ActiveNode:get_text_at(0):update_text(string.format("%.3f", Engine.scene:get_transform(_G.selected_transform).rotation.x))
        time_since_text_update = 0
        frame_count = 0
	end
end
function update_rotation_y_display()
	time_since_text_update = time_since_text_update + dt

	if time_since_text_update > 0.1 then
        gui.ActiveNode:get_text_at(0):update_text(string.format("%.3f", Engine.scene:get_transform(_G.selected_transform).rotation.y))
        time_since_text_update = 0
        frame_count = 0
	end
end
function update_rotation_z_display()
	time_since_text_update = time_since_text_update + dt

	if time_since_text_update > 0.1 then
        gui.ActiveNode:get_text_at(0):update_text(string.format("%.3f", Engine.scene:get_transform(_G.selected_transform).rotation.z))
        time_since_text_update = 0
        frame_count = 0
	end
end
function update_scale_x_display()
	time_since_text_update = time_since_text_update + dt

	if time_since_text_update > 0.1 then
        gui.ActiveNode:get_text_at(0):update_text(string.format("%.3f", Engine.scene:get_transform(_G.selected_transform).scale.x))
        time_since_text_update = 0
        frame_count = 0
	end
end
function update_scale_y_display()
	time_since_text_update = time_since_text_update + dt

	if time_since_text_update > 0.1 then
        gui.ActiveNode:get_text_at(0):update_text(string.format("%.3f", Engine.scene:get_transform(_G.selected_transform).scale.y))
        time_since_text_update = 0
        frame_count = 0
	end
end
function update_scale_z_display()
	time_since_text_update = time_since_text_update + dt

	if time_since_text_update > 0.1 then
        gui.ActiveNode:get_text_at(0):update_text(string.format("%.3f", Engine.scene:get_transform(_G.selected_transform).scale.z))
        time_since_text_update = 0
        frame_count = 0
	end
end

local drag_mode
function set_drag_mode_translation_x()
    drag_mode = "tx"
end
function set_drag_mode_translation_y()
    drag_mode = "ty"
end
function set_drag_mode_translation_z()
    drag_mode = "tz"
end
function set_drag_mode_rotation_x()
    drag_mode = "rx"
end
function set_drag_mode_rotation_y()
    drag_mode = "ry"
end
function set_drag_mode_rotation_z()
    drag_mode = "rz"
end
function set_drag_mode_scale_x()
    drag_mode = "sx"
end
function set_drag_mode_scale_y()
    drag_mode = "sy"
end
function set_drag_mode_scale_z()
    drag_mode = "sz"
end

function color_slider_hovered()
    gui.ActiveNode:get_quad_at(0).color = Vector.new4(0.4, 0.4, 0.4, 1.0)
end
function color_slider_unhovered()
    gui.ActiveNode:get_quad_at(0).color = Vector.new4(0.35, 0.35, 0.35, 1.0)
end

local original_cursor_x
local original_translation
local original_rotation
local original_scale
function begin_drag()
    original_cursor_x = Engine.client.cursor_position.x
    original_translation = Engine.scene:get_transform(_G.selected_transform).translation
    original_rotation = Engine.scene:get_transform(_G.selected_transform).rotation
    original_scale = Engine.scene:get_transform(_G.selected_transform).scale
end
function update_drag()
    local transform = Engine.scene:get_transform(_G.selected_transform)
    local delta = Engine.client.cursor_position.x - original_cursor_x

    if drag_mode == "tx" then
        transform.translation = Vector.new3(
            original_translation.x + delta * 0.005,
            transform.translation.y,
            transform.translation.z
        )
    elseif drag_mode == "ty" then
        transform.translation = Vector.new3(
            transform.translation.x,
            original_translation.y + delta * 0.005,
            transform.translation.z
        )
    elseif drag_mode == "tz" then
        transform.translation = Vector.new3(
            transform.translation.x,
            transform.translation.y,
            original_translation.z + delta * 0.005
        )
    elseif drag_mode == "rx" then
        transform.rotation = Vector.new3(
            original_rotation.x + delta * 0.005,
            transform.rotation.y,
            transform.rotation.z
        )
    elseif drag_mode == "ry" then
        transform.rotation = Vector.new3(
            transform.rotation.x,
            original_rotation.y + delta * 0.005,
            transform.rotation.z
        )
    elseif drag_mode == "rz" then
        transform.rotation = Vector.new3(
            transform.rotation.x,
            transform.rotation.y,
            original_rotation.z + delta * 0.005
        )
    elseif drag_mode == "sx" then
        transform.scale = Vector.new3(
            original_scale.x + delta * 0.005,
            transform.scale.y,
            transform.scale.z
        )
    elseif drag_mode == "sy" then
        transform.scale = Vector.new3(
            transform.scale.x,
            original_scale.y + delta * 0.005,
            transform.scale.z
        )
    elseif drag_mode == "sz" then
        transform.scale = Vector.new3(
            transform.scale.x,
            transform.scale.y,
            original_scale.z + delta * 0.005
        )
    end
end

local collapsed = true
function collapse()
    if not collapsed then
        collapsed = true
        gui.ActiveNode:get_parent():set_height("Absolute", 20.0)
    else
        collapsed = false
        gui.ActiveNode:get_parent():set_height("Absolute", 215.0)
    end
end