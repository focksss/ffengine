local initial_mouse_pos = Vector.new()
local initial_value = Vector.new()

local resize_called_last_tick = false
local resize_called_this_tick = false

local right_area_node_index = 6
local scene_view_node_index = 9

local minimize_button_index = 2
local maximize_button_index = 3

local scene_graph_index = 8
local scene_graph_text_indices_start = 2

local target_cursor_icon = CursorIcon.Default

function Awake()
	build_graph()
end

function horizontal_resize_cursor()
	target_cursor_icon = CursorIcon.EResize
end
function vertical_resize_cursor()
	target_cursor_icon = CursorIcon.NResize
end
function hover_cursor()
	target_cursor_icon = CursorIcon.Pointer
end
function nw_resize_cursor()
	target_cursor_icon = CursorIcon.NwResize
end
function sw_resize_cursor()
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
	Engine.renderer:gui(0).ActiveNode.quad.color = Vector.new4(1.0, 0.3, 0.3, 1.0)
end

function color_hovered()
	Engine.renderer:gui(0).ActiveNode.quad.color = Vector.new4(2.0, 2.0, 2.0, 0.15)
end

function color_unhovered()
	Engine.renderer:gui(0).ActiveNode.quad.color = Vector.new4(1.0, 1.0, 1.0, 0.0)
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

	local gui = Engine.renderer:gui(0)
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
        Engine.renderer:gui(0).ActiveNode.text:update_text(string.format("FPS: %.1f", fps))
        time_since_fps_update = 0
        frame_count = 0
	end
end

local stored_window_size = Vector.new2(0, 0)
function Update()
	local gui = Engine.renderer:gui(0)
	local window_size = Engine.client.window_size
	local right_area_node = gui:get_node(right_area_node_index)

	local maximized = Engine.client.maximized
	gui:get_node(minimize_button_index).hidden = not maximized
	gui:get_node(maximize_button_index).hidden = maximized

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

	right_area_node.scale = Vector.new2(right_area_node.scale.x, window_size.y - 40)

	local scene_view_node = gui:get_node(scene_view_node_index)
	scene_view_node.scale = Vector.new2(window_size.x - right_area_node.scale.x, window_size.y - 40)

end

local expanded_entities = {}
local node_to_entity_map = {}
local graph_level = 0
local used_text_count = 0
local darker = false
function toggle_graph_node() 
	local gui = Engine.renderer:gui(0)
	local clicked_node = gui.ActiveNode
	local clicked_node_index = clicked_node.index
	
	local entity_index = node_to_entity_map[clicked_node_index]
	
	if entity_index then
		expanded_entities[entity_index] = not expanded_entities[entity_index]
		
		build_graph()
	end
end
function build_graph()
	local gui = Engine.renderer:gui(0)
	local scene_graph_node = gui:get_node(scene_graph_index)

	-- remove existing children + quads
	local num_children = #scene_graph_node.children_indices
	for i = num_children, 1, -1 do
		local child_index = scene_graph_node:get_child_index(i - 1)
		gui:destroy_quad(gui:get_node(child_index).quad_index)
		gui:destroy_node(child_index)
		scene_graph_node:remove_child_index_at(i - 1)
	end
	
	--- reset mapping for expansion tracking (root expanded)
	node_to_entity_map = {}
	if expanded_entities[0] == nil then
		expanded_entities[0] = true
	end

	-- reset text usage counter
	used_text_count = 0

	-- reset length
	graph_level = 0

	-- reset alternating pattern
	darker = false
	
	local root_entity = Engine.scene:get_entity(0)
	build_graph_recursive(root_entity, 0, 0, scene_graph_node)
end
function build_graph_recursive(entity, entity_index, depth, parent_gui_node)
	local gui = Engine.renderer:gui(0)
	
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
	if has_children then
		if is_expanded then
			display_name = "> " .. display_name .. "      "
		else
			display_name = display_name .. "      "
		end
	end

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
	node.quad_index = quad_index

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
	text.position = Vector.new2(0.01, 0.2)
	node.text_index = text_index
	used_text_count = used_text_count + 1

	-- format
	node.position = Vector.new2(depth * 20, -graph_level * 20)
	node.absolute_position_x = true
	node.absolute_position_y = true

	node.scale = Vector.new2(1.0, 20)
	node.absolute_scale_x = false
	node.absolute_scale_y = true
	node:set_anchor_point(AnchorPoint.TopLeft)

	-- add as child of parent
	parent_gui_node:add_child_index(node_index)
	
	graph_level = graph_level + 1

	-- recursively process children
	if is_expanded then
		for i = 1, #children do
			local child_entity_index = children[i]
			local child_entity = Engine.scene:get_entity(child_entity_index)
			build_graph_recursive(child_entity, child_entity_index, depth + 1, parent_gui_node)
		end
	end
end