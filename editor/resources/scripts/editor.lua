local target_cursor_icon
local has_set_target_cursor = false

local stored_window_size = Vector.new2(0, 0)

local resize_called_last_tick = false
local resize_called_this_tick = false

local gui

	local root_node
		local titlebar_node
			local close_button_node
			local toggle_maximize_button_node
			local minimize_window_button_node
		local right_area_node
			local scene_graph_area_node
				local scene_graph_root_node
		local scene_view_area_node

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
		right_area_node = root_node:get_child(1)
			scene_graph_area_node = right_area_node:get_child(0)
				scene_graph_root_node = scene_graph_area_node:get_child(0)
		scene_view_area_node = root_node:get_child(2)

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
	
	build_graph()

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

	local scene_viewport = Engine.renderer.scene_renderer.viewport
	if 
		not resize_called_this_tick and
		not resize_called_last_tick and
		(scene_viewport.width ~= scene_view_area_node.size.x or
		 scene_viewport.height ~= scene_view_area_node.size.y + 40)
	then
		scene_viewport.width = scene_view_area_node.size.x
		scene_viewport.height = scene_view_area_node.size.y + 40
		
		Engine.client.flags.reload_rendering_queued = true
	end

	resize_called_last_tick = resize_called_this_tick
	resize_called_this_tick = false

end

local expanded_entities = {}
local node_to_entity_map = {}
local graph_height = 0
local darker = false
local graph_scroll_pixels = 10
local graph_entity_height = 20
local graph_child_indent = 20
function toggle_graph_node() 
	
	local clicked_node = gui.ActiveNode
	local clicked_node_index = clicked_node.index
	
	local entity_index = node_to_entity_map[clicked_node_index]
	
	if entity_index then
		expanded_entities[entity_index] = not expanded_entities[entity_index]
		
		build_graph()
	end
end
function build_graph()
	-- remove existing children + quads
	local num_children = #scene_graph_root_node.children_indices
	for i = num_children, 1, -1 do
		local child_index = scene_graph_root_node:get_child_index(i - 1)
		gui:destroy_node(child_index)
		scene_graph_root_node:remove_child_index_at(i - 1)
	end
	
	--- reset mapping for expansion tracking (root expanded)
	node_to_entity_map = {}
	if expanded_entities[0] == nil then
		expanded_entities[0] = true
	end

	-- reset height
	graph_height = 0

	-- reset alternating pattern
	darker = false
	
	local root_entity = Engine.scene:get_entity(0)
	build_graph_recursive(root_entity, 0, 0, scene_graph_root_node)

	--[[
	scene_graph_scroll_bar_node.scale = Vector.new2(
		scene_graph_scroll_bar_node.scale.x, 
		math.min(1.0, (scene_graph_area_node.scale.y * right_area_node.scale.y) / graph_height) --- visible pixels / total pixels
	)
	--]]
end
function build_graph_recursive(entity, entity_index, depth, parent_gui_node)
	
	-- node
	local node_index = gui.num_nodes
	gui:add_node(parent_gui_node.index)
	local node = gui:get_node(node_index)
	node:add_left_tap_action("toggle_graph_node", 0)

	--- map
	node_to_entity_map[node_index] = entity_index
	local is_expanded = expanded_entities[entity_index]
	local display_name = entity.name
	local children = entity.children_indices
	local has_children = #children > 0	

	-- quad
	local quad_index = 2
	if darker then quad_index = 3 end
	node:add_element_index(quad_index)
	--- add the expanded/collapsed visual image after so its drawn on top
	--[[
	if has_children then
		if is_expanded then
			node:add_element_index(10)
		else
			node:add_element_index(9)
		end
	end
	---]]

	-- format
	node:set_x("Pixels", depth * graph_child_indent)
	node:set_y("Pixels", graph_height)

	node:set_width("Factor", 1.0)
	node:set_height("Absolute", 20.0)

	node:set_anchor_point(AnchorPoint.TopLeft)

	-- add as child of parent
	parent_gui_node:add_child_index(node_index)
	
	graph_height = graph_height + graph_entity_height

	-- recursively process children
	if is_expanded then
		for i = 1, #children do
			local child_entity_index = children[i]
			local child_entity = Engine.scene:get_entity(child_entity_index)
			build_graph_recursive(child_entity, child_entity_index, depth + 1, parent_gui_node)
		end
	end
end

local time_since_fps_update = 0
local frame_count = 0
function update_fps()
	frame_count = frame_count + 1
	time_since_fps_update = time_since_fps_update + dt

	if time_since_fps_update > 1.0 then 
		local fps = frame_count / time_since_fps_update
        gui.ActiveNode:get_text_at(0):update_text(string.format("FPS: %.1f", fps))
        time_since_fps_update = 0
        frame_count = 0
	end
end

function resize_right_area()
	resize_called_this_tick = true

	local window_size = Engine.client.window_size
	right_area_node:set_width("Absolute", window_size.x - Engine.client.cursor_position.x)
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