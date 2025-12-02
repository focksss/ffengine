local target_cursor_icon
local has_set_target_cursor = false

local resize_called_last_tick = false
local resize_called_this_tick = false

local gui

local top_bar_node
	local minimize_button_node
	local maximize_button_node

local right_area_node
	local scene_graph_area_node
		local scene_graph_parent_node

		local scene_graph_text_indices_start
		local scene_graph_scroll_bar_node
		
local scene_view_node

function Awake()
	target_cursor_icon = CursorIcon.Default
	
	gui = Engine.renderer:gui(0)

	top_bar_node = gui:get_root(0)
		minimize_button_node = top_bar_node:get_child(1)
		maximize_button_node = top_bar_node:get_child(2)

	right_area_node = gui:get_root(1)
		scene_graph_area_node = right_area_node:get_child(1)
			scene_graph_parent_node = scene_graph_area_node:get_child(0)
			scene_graph_scroll_bar_node = scene_graph_area_node:get_child(1)

			scene_graph_text_indices_start = gui.num_texts

	scene_view_node = gui:get_root(2)
	
	build_graph()
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

function close_window() 
    Engine.client.flags.close_requested = true
end

function recompile() 
    Engine.client.flags.reload_rendering_queued = true
    Engine.client.flags.reload_scripts_queued = true
end

function close_hovered()
	gui.ActiveNode:get_quad(0).color = Vector.new4(1.0, 0.3, 0.3, 1.0)
end

function color_hovered()
	gui.ActiveNode:get_quad(0).color = Vector.new4(2.0, 2.0, 2.0, 0.15)
end

function color_unhovered()
	gui.ActiveNode:get_quad(0).color = Vector.new4(1.0, 1.0, 1.0, 0.0)
end

function drag_window() 
	Engine.client:drag_window()
end

function maximize()
	Engine.client.maximized = true
end
function minimize()
	Engine.client.maximized = false
end
function minimize_window()
	Engine.client.minimized = true
end

function resize_right_area()
	resize_called_this_tick = true

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
        gui.ActiveNode:get_text(0):update_text(string.format("FPS: %.1f", fps))
        time_since_fps_update = 0
        frame_count = 0
	end
end

local stored_window_size = Vector.new2(0, 0)
function Update()
	if gui == nil then return end
	
	local window_size = Engine.client.window_size

	local maximized = Engine.client.maximized
	minimize_button_node.hidden = not maximized
	maximize_button_node.hidden = maximized

	has_set_target_cursor = false
	Engine.client:set_cursor_icon(target_cursor_icon)
	target_cursor_icon = CursorIcon.Default

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

	right_area_node.scale = Vector.new2(right_area_node.scale.x, window_size.y - top_bar_node.scale.y)

	scene_view_node.scale = Vector.new2(window_size.x - right_area_node.scale.x, window_size.y - top_bar_node.scale.y)

end

local expanded_entities = {}
local node_to_entity_map = {}
local graph_height = 0
local used_text_count = 0
local darker = false
local graph_scroll_pixels = 10
local graph_entity_height = 20
local graph_child_indent = 20
function MouseScrolled()
	local total_pixels = scene_graph_area_node.scale.y * right_area_node.scale.y
	if gui:is_node_hovered(scene_graph_area_node.index) then
		local max_content_scroll = graph_height - total_pixels
		local max_scrollbar_travel = (1.0 - scene_graph_scroll_bar_node.scale.y) * total_pixels
		
		scene_graph_scroll_bar_node.position = Vector.new2(
			scene_graph_scroll_bar_node.position.x, 
			math.max(
				math.min(0, scene_graph_scroll_bar_node.position.y + Engine.client.scroll_delta.y * graph_scroll_pixels), --- clamp to top
				-max_scrollbar_travel --- clamp to bottom
			)
		)
		
		-- Map scrollbar position to content position
		local scroll_ratio = max_scrollbar_travel > 0 and (-scene_graph_scroll_bar_node.position.y / max_scrollbar_travel) or 0
		scene_graph_parent_node.position = Vector.new2(
			0,
			scroll_ratio * max_content_scroll
		)
	end
end
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
	local num_children = #scene_graph_parent_node.children_indices
	for i = num_children, 1, -1 do
		local child_index = scene_graph_parent_node:get_child_index(i - 1)
		gui:destroy_quad(gui:get_node(child_index):get_quad_index(0))
		gui:destroy_node(child_index)
		scene_graph_parent_node:remove_child_index_at(i - 1)
	end
	
	--- reset mapping for expansion tracking (root expanded)
	node_to_entity_map = {}
	if expanded_entities[0] == nil then
		expanded_entities[0] = true
	end

	-- reset text usage counter
	used_text_count = 0

	-- reset height
	graph_height = 0

	-- reset alternating pattern
	darker = false
	
	local root_entity = Engine.scene:get_entity(0)
	build_graph_recursive(root_entity, 0, 0, scene_graph_parent_node)

	scene_graph_scroll_bar_node.scale = Vector.new2(
		scene_graph_scroll_bar_node.scale.x, 
		math.min(1.0, (scene_graph_area_node.scale.y * right_area_node.scale.y) / graph_height) --- visible pixels / total pixels
	)
end
function build_graph_recursive(entity, entity_index, depth, parent_gui_node)
	
	
	-- node
	local node_index = gui.num_nodes
	gui:add_node()
	local node = gui:get_node(node_index)
	node:add_left_tap_action("toggle_graph_node", 0)

	--- map
	node_to_entity_map[node_index] = entity_index
	local is_expanded = expanded_entities[entity_index]
	local display_name = entity.name
	local children = entity.children_indices
	local has_children = #children > 0	

	-- quad
	local quad_index = gui.num_quads
	gui:add_quad()
	local quad = gui:get_quad(quad_index)
	if darker then
		quad.color = Vector.new4(0.25, 0.25, 0.25, 1.0)
		darker = false
	else
		quad.color = Vector.new4(0.3, 0.3, 0.3, 1.0)
		darker = true
	end
	quad.corner_radius = 5.0
	node:add_quad_index(quad_index)
	--- add the expanded/collapsed visual quad after so its drawn on top
	if has_children then
		if is_expanded then
			node:add_quad_index(10)
		else
			node:add_quad_index(9)
		end
	end

	-- reuse or create text
	local text_index = scene_graph_text_indices_start + used_text_count
	if text_index >= gui.num_texts then
		gui:add_text(display_name)
	else
		local text = gui:get_text(text_index)
		text:update_text(display_name)
	end
	local text = gui:get_text(text_index)
	text.font_size = 15.0
	text.auto_wrap_distance = 1000.0
	text.position = Vector.new2(20, 0.2)
	text.absolute_position_x = true
	node:add_text_index(text_index)
	used_text_count = used_text_count + 1

	-- format
	node.position = Vector.new2(depth * graph_child_indent, -graph_height)
	node.absolute_position_x = true
	node.absolute_position_y = true

	node.scale = Vector.new2(1.0, 20)
	node.absolute_scale_x = false
	node.absolute_scale_y = true
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