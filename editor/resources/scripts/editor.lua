local initial_mouse_pos = Vector.new()
local initial_value = Vector.new()

local resize_called_last_tick = false
local resize_called_this_tick = false

local right_area_node_index = 3
local scene_view_node_index = 5

function drag_resize_north()
	resize_called_this_tick = true
	Engine.client:drag_resize_window(ResizeDirection.North)
end
function drag_resize_south()
	resize_called_this_tick = true
	Engine.client:drag_resize_window(ResizeDirection.South)
end
function drag_resize_east()
	resize_called_this_tick = true
	Engine.client:drag_resize_window(ResizeDirection.East)
end
function drag_resize_west()
	resize_called_this_tick = true
	Engine.client:drag_resize_window(ResizeDirection.West)
end
function drag_resize_northeast()
	resize_called_this_tick = true
	Engine.client:drag_resize_window(ResizeDirection.NorthEast)
end
function drag_resize_northwest()
	resize_called_this_tick = true
	Engine.client:drag_resize_window(ResizeDirection.NorthWest)
end
function drag_resize_southeast()
	resize_called_this_tick = true
	Engine.client:drag_resize_window(ResizeDirection.SouthEast)
end
function drag_resize_southwest()
	resize_called_this_tick = true
	Engine.client:drag_resize_window(ResizeDirection.SouthWest)
end

function close_window() 
    Engine.client.flags.close_requested = true
end

function recompile() 
    Engine.client.flags.reload_rendering_queued = true
    Engine.client.flags.reload_scripts_queued = true
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

local stored_window_size = Vector.new2(0, 0)
function Update()
	local gui = Engine.renderer.gui
	local window_size = Engine.client.window_size
	local right_area_node = gui:get_node(right_area_node_index)

	if not resize_called_this_tick and resize_called_last_tick then
		local scene_viewport = Engine.renderer.scene_renderer.viewport

		scene_viewport.width = window_size.x - right_area_node.scale.x
		scene_viewport.height = window_size.y

		stored_window_size = window_size
		Engine.client.flags.reload_rendering_queued = true
	elseif 
		not resize_called_this_tick and
		not resize_called_last_tick and
		(window_size.x ~= stored_window_size.x or
		 window_size.y ~= stored_window_size.y)
	then
		local scene_viewport = Engine.renderer.scene_renderer.viewport
		scene_viewport.width = window_size.x - right_area_node.scale.x
		scene_viewport.height = window_size.y
		
		stored_window_size = window_size
		Engine.client.flags.reload_rendering_queued = true
	end
	
	resize_called_last_tick = resize_called_this_tick
	resize_called_this_tick = false

	right_area_node.scale = Vector.new2(right_area_node.scale.x, window_size.y - 40)

	local scene_view_node = gui:get_node(scene_view_node_index)
	scene_view_node.scale = Vector.new2(window_size.x - right_area_node.scale.x, window_size.y - 40)
end