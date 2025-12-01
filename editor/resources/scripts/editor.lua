local target_cursor_icon
local has_set_target_cursor = false

local resize_called_last_tick = false
local resize_called_this_tick = false

local gui

	local root_node

		local titlebar_node

			local close_button_node
			local toggle_maximize_button_node
			local minimize_window_button_node

	local resize_root_node
		local resize_top_root_node
			local resize_top_node
			local resize_top_left_node
			local resize_top_right_node
		local resize_bottom_root_node
			local resize_bottom_node
			local resize_bottom_left_node
			local resize_bottom_right_node
		local resize_left_node
		local resize_right_node

function Awake()
	target_cursor_icon = CursorIcon.Default
	
	gui = Engine.renderer:gui(0)

	root_node = gui:get_root(0)

		titlebar_node = root_node:get_child(0)

			close_button_node = titlebar_node:get_child(0)
			toggle_maximize_button_node = titlebar_node:get_child(1)
			minimize_window_button_node = titlebar_node:get_child(1)

	resize_root_node = gui:get_root(1)
		resize_top_root_node = resize_root_node:get_child(0)
			resize_top_left_node = resize_top_root_node:get_child(0)
			resize_top_right_node = resize_top_root_node:get_child(1)
			resize_top_node = resize_top_root_node:get_child(2)
		resize_bottom_root_node = resize_root_node:get_child(1)
			resize_bottom_left_node = resize_bottom_root_node:get_child(0)
			resize_bottom_right_node = resize_bottom_root_node:get_child(1)
			resize_bottom_node = resize_bottom_root_node:get_child(2)
		resize_left_node = resize_root_node:get_child(2)
		resize_right_node = resize_root_node:get_child(3)
	
end

function Update()
	if gui == nil then return end
	
	local window_size = Engine.client.window_size

	local maximized = Engine.client.maximized
	local toggle_maximize_button_image_index = 0
	if maximized then toggle_maximize_button_image_index = 1 end
	toggle_maximize_button_node:set_element_index_at_to(0, toggle_maximize_button_image_index)

	has_set_target_cursor = false
	Engine.client:set_cursor_icon(target_cursor_icon)
	target_cursor_icon = CursorIcon.Default

end

function drag_window() 
	Engine.client:drag_window()
end
function close_window() 
    Engine.client.flags.close_requested = true
end
function toggle_maximize()
	Engine.client.maximized = not Engine.client.maximized
end
function minimize_window()
	Engine.client.minimized = true
end

function close_hovered()
	local image = gui.ActiveNode:get_image_at(0)
	image.additive_tint = Vector.new4(1.0, 0.3, 0.3, 1.0)
end

function color_image_hovered()
	local image = gui.ActiveNode:get_image_at(0)
	image.additive_tint = Vector.new4(2.0, 2.0, 2.0, 0.15)
end

function color_image_unhovered()
	local image = gui.ActiveNode:get_image_at(0)
	image.additive_tint = Vector.new4(2.0, 2.0, 2.0, 0.0)
end

function horizontal_resize_cursor()
	if has_set_target_cursor then
		return
	else
		has_set_target_cursor = true
	end
	target_cursor_icon = CursorIcon.EResize
end
function vertical_resize_cursor()
	if has_set_target_cursor then
		return
	else
		has_set_target_cursor = true
	end
	target_cursor_icon = CursorIcon.NResize
end
function hover_cursor()
	if has_set_target_cursor then
		return
	else
		has_set_target_cursor = true
	end
	target_cursor_icon = CursorIcon.Pointer
end
function nw_resize_cursor()
	if has_set_target_cursor then
		return
	else
		has_set_target_cursor = true
	end
	target_cursor_icon = CursorIcon.NwResize
end
function sw_resize_cursor()
	if has_set_target_cursor then
		return
	else
		has_set_target_cursor = true
	end
	target_cursor_icon = CursorIcon.SwResize
end

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