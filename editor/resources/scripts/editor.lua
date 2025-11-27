local initial_mouse_pos = Vector.new()
local initial_value = Vector.new()

local resize_called_last_tick = false
local resize_called_this_tick = false

local right_area_node_index = 3
local scene_view_node_index = 5

function close_window() 
    Engine.client.flags.close_requested = true
end

function recompile() 
    Engine.client.flags.recompile_queued = true
end

function close_hovered()
	Engine.renderer.gui.ActiveNode.quad:set_color(1.0, 0.3, 0.3, 1.0)
end

function color_hovered()
	Engine.renderer.gui.ActiveNode.quad:set_color(0.3, 0.3, 0.3, 1.0)
end

function color_unhovered()
	Engine.renderer.gui.ActiveNode.quad:set_color(0.0, 0.0, 0.0, 0.0)
end

function drag_window() 
	Engine.client:drag_window()
end

function resize_right_area()
	resize_called_this_tick = true

	local gui = Engine.renderer.gui
	local right_area_node = gui:get_node(right_area_node_index)
	local window_size = Engine.client.window_size
	right_area_node.scale = Vector.new2(
		window_size.x 
		- Engine.client.cursor_position.x
		+ 5, right_area_node.scale.y
	) 
	
	local scene_viewport = Engine.renderer.scene_renderer.viewport
	scene_viewport.width = window_size.x - right_area_node.scale.x
end

local time_since_fps_update = 0
local frame_count = 0
function update_fps()
	frame_count = frame_count + 1
	time_since_fps_update = time_since_fps_update + dt

	if time_since_fps_update > 1.0 then 
		local fps = frame_count / time_since_fps_update
        Engine.renderer.gui.ActiveNode.text:update_text(string.format("FPS: %.1f", fps))
        time_since_fps_update = 0
        frame_count = 0
	end
end

function Update()
	if resize_called_last_tick and not resize_called_this_tick then
		Engine.client.flags.recompile_queued = true
	end
	
	resize_called_last_tick = resize_called_this_tick
	resize_called_this_tick = false

	local window_size = Engine.client.window_size

	local gui = Engine.renderer.gui
	local right_area_node = gui:get_node(right_area_node_index)
	right_area_node.scale = Vector.new2(right_area_node.scale.x, window_size.y - 40)

	local scene_view_node = gui:get_node(scene_view_node_index)
	scene_view_node.scale = Vector.new2(window_size.x - right_area_node.scale.x, window_size.y - 40)
end