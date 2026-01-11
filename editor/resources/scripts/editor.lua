_G.Editor = {}

_G.target_cursor_icon = CursorIcon.Default
local has_set_target_cursor = false

local resize_called_last_tick = false
local resize_called_this_tick = false

local gui

	local root_node
		local titlebar_node
			local toggle_maximize_button_node
			local middle_buttons_node
				local play_pause_node
					local play_node
					local pause_node
		local right_area_node
			local scene_graph_area_node
				local scene_graph_root_node
					local graph_height = 0
				local scene_graph_scroll_bar_node
		local scene_view_area_node

function Awake()
	gui = Engine.renderer:gui(0)

	root_node = gui:get_root(0)
		titlebar_node = root_node:get_child(0)
			toggle_maximize_button_node = titlebar_node:get_child(1)
			middle_buttons_node = titlebar_node:get_child(3)
				play_pause_node = middle_buttons_node:get_child(0)
					play_node = play_pause_node:get_child(0)
					pause_node = play_pause_node:get_child(1)
		right_area_node = root_node:get_child(1)
			scene_graph_area_node = right_area_node:get_child(0)
				scene_graph_root_node = scene_graph_area_node:get_child(0)
				scene_graph_scroll_bar_node = scene_graph_area_node:get_child(1):get_child(0)
		scene_view_area_node = root_node:get_child(2)
	
	build_graph()

end


function Update()
	if gui == nil then return end

	local maximized = Engine.client.maximized
	local toggle_maximize_button_image_index = 1
	if maximized then toggle_maximize_button_image_index = 0 end
	toggle_maximize_button_node:set_element_index_at_to(0, toggle_maximize_button_image_index)

	has_set_target_cursor = false
	Engine.client:set_cursor_icon(target_cursor_icon)
	target_cursor_icon = CursorIcon.Default

	local scene_viewport = Engine.renderer.scene_renderer.viewport
	if 
		not resize_called_this_tick and
		not resize_called_last_tick and
		(scene_viewport.width ~= scene_view_area_node.size.x or
		 scene_viewport.height ~= scene_view_area_node.size.y)
	then
		scene_viewport.width = scene_view_area_node.size.x
		scene_viewport.height = scene_view_area_node.size.y

		scene_viewport.y = 30
		
		Engine.client.flags.reload_rendering_queued = true
	end

	scene_graph_scroll_bar_node:set_height("Factor",
		math.min(1.0, scene_graph_area_node.size.y / graph_height) --- visible pixels / total pixels
	)

	resize_called_last_tick = resize_called_this_tick
	resize_called_this_tick = false

end

local passively_selected_entities = {}
local entity_to_node_map = {}
local selected_node = -1
local selected_entity = 0
_G.Editor.deselect = function()
	for _, entity_index in ipairs(passively_selected_entities) do
		local node_index = entity_to_node_map[entity_index]
		if node_index ~= nil then
			gui:get_node(node_index):get_child(2):remove_element_index_at(1)
		end
	end
	passively_selected_entities = {}
	
	if selected_node > -1 then
		local node = gui:get_node(selected_node)
		node:remove_element_index_at(1)
		node:get_child(2):remove_element_index_at(1)
	end
end
_G.Editor.select_entity = function(select_entity_index)
	_G.Editor.deselect()

	local node_index = entity_to_node_map[select_entity_index]
	if node_index ~= nil then
		local node = gui:get_node(node_index)
		node:add_element_index(8)
		node:get_child(2):add_element_index(10)
		selected_node = node.index
	end
	selected_entity = select_entity_index

	local entity = Engine.scene:get_entity(selected_entity)
	local parent = entity.parent
	passively_selected_entities[1] = selected_entity
	local num_passively_selected_entities = 1

	while parent.index ~= parent.parent.index do
		node_index = entity_to_node_map[parent.index]
		if node_index ~= nil then
			gui:get_node(node_index):get_child(2):add_element_index(10)
		end
		num_passively_selected_entities = num_passively_selected_entities + 1
		passively_selected_entities[num_passively_selected_entities] = parent.index
		parent = parent.parent
	end
	gui:get_node(entity_to_node_map[parent.index]):get_child(2):add_element_index(10)
	num_passively_selected_entities = num_passively_selected_entities + 1
	passively_selected_entities[num_passively_selected_entities] = parent.index
end
function select_entity()
	_G.Editor.select_entity(_G.node_to_entity_map[gui.ActiveNode.index])
end
function import_model_as_child() 
	local entity_index = _G.node_to_entity_map[gui.ActiveNode.index]
	Engine.scene:load_model(entity_index)
	build_graph()
end
local expanded_entities = {}
_G.node_to_entity_map = {}
_G.node_to_render_components_map = {}
_G.node_to_rigid_body_map = {}
_G.node_to_transform_map = {}
local darker = false
local graph_scroll_pixels = 10
local graph_entity_height = 20
local graph_child_indent = 20
local graph_owned_element_indices = {}
local current_used_text_count = 0
local graph_owned_node_indices = {}
local current_used_node_count = 0
function MouseScrolled()
	if gui:is_node_hovered(scene_graph_area_node.index) then
		local total_pixels = scene_graph_area_node.size.y
		local max_content_scroll = graph_height - total_pixels
		local max_scrollbar_travel = total_pixels - scene_graph_scroll_bar_node.size.y
		
		local current_travel = scene_graph_scroll_bar_node.position.y - scene_graph_area_node.position.y

		scene_graph_scroll_bar_node:set_y("Pixels",
			math.min(math.max(0.0, current_travel - Engine.client.scroll_delta.y * graph_scroll_pixels), max_scrollbar_travel)
		)
		
		current_travel = scene_graph_scroll_bar_node.position.y - scene_graph_area_node.position.y

		-- Map scrollbar position to content position
		local scroll_ratio = max_scrollbar_travel > 0 and (-current_travel / max_scrollbar_travel) or 0
		scene_graph_root_node:set_y("Pixels", scroll_ratio * max_content_scroll)
	end
end

function toggle_graph_node() 
	
	local clicked_node = gui.ActiveNode
	local clicked_node_index = clicked_node.index
	
	local entity_index = _G.node_to_entity_map[clicked_node_index]
	
	if entity_index then
		expanded_entities[entity_index] = not expanded_entities[entity_index]
		
		build_graph()
	end
end
function build_graph()
	-- reset node tree
	scene_graph_root_node:clear_children()
	
	--- reset mappings (and start root expanded)
	selected_node = -1
	_G.node_to_entity_map = {}
	entity_to_node_map = {}
	_G.node_to_render_components_map = {}
	_G.node_to_rigid_body_map = {}
	_G.node_to_transform_map = {}
	if expanded_entities[0] == nil then
		expanded_entities[0] = true
	end

	-- reset object counters
	current_used_text_count = 0
	current_used_node_count = 0

	-- reset height
	graph_height = 0

	-- reset alternating pattern
	darker = false
	
	local root_entity = Engine.scene:get_entity(0)
	build_graph_recursive(root_entity, 0, 0, scene_graph_root_node)

	for _, entity_index in ipairs(passively_selected_entities) do
		local node_index = entity_to_node_map[entity_index]
		if node_index ~= nil then
			gui:get_node(entity_to_node_map[entity_index]):get_child(2):add_element_index(10)
		end
	end

	scene_graph_root_node:set_height("Absolute", graph_height)
end
local function get_next_graph_node_index()
	local current_owned_node_count = #graph_owned_node_indices
	if current_used_node_count >= current_owned_node_count then
		graph_owned_node_indices[current_owned_node_count + 1] = gui:add_node(scene_graph_root_node.index)
	end
	current_used_node_count = current_used_node_count + 1
	return graph_owned_node_indices[current_used_node_count]
end
local function get_next_graph_text_index(text_string)
	local current_owned_element_count = #graph_owned_element_indices
	if current_used_text_count >= current_owned_element_count then
		graph_owned_element_indices[current_owned_element_count + 1] = gui:add_text(text_string)
	else
		local text = gui:get_text(graph_owned_element_indices[current_used_text_count + 1])
		text:update_text(text_string)
	end
	return graph_owned_element_indices[current_used_text_count + 1]
end
function build_graph_recursive(entity, entity_index, depth)
	
	-- entity root node
	local node_index = get_next_graph_node_index()
	local node = gui:get_node(node_index)
	node:reset()
	node:add_left_up_action("open_entity_editor", 3)
	node:add_left_up_action("select_entity", 0)
	node:add_right_up_action("import_model_as_child", 0)
	node:add_hover_action("hover_cursor", 0)
	node:set_x("Pixels", depth * graph_child_indent)
	node:set_y("Pixels", graph_height)
	node:set_width("Factor", 1.0)
	node:set_height("Absolute", 20.0)
	node:set_anchor_point(AnchorPoint.TopLeft)
	scene_graph_root_node:add_child_index(node_index)
	-- text node
	local text_node_index = get_next_graph_node_index()
	local text_node = gui:get_node(text_node_index)
	text_node:reset()
	text_node:set_x("Pixels", 40)
	text_node:set_width("Factor", 1.0)
	text_node:set_height("Factor", 1.0)
	node:add_child_index(text_node_index)
	-- toggle expand button node
	local button_node_index = get_next_graph_node_index()
	local button_node = gui:get_node(button_node_index)
	button_node:reset()
	button_node:add_left_up_action("toggle_graph_node", 0)
	button_node:add_hover_action("hover_cursor", 0)
	button_node:set_x("Pixels", 0)
	button_node:set_width("Absolute", 20.0)
	button_node:set_height("Absolute", 20.0)
	node:add_child_index(button_node_index)
	-- object icon node
	local object_node_index = get_next_graph_node_index()
	local object_node = gui:get_node(object_node_index)
	object_node:reset()
	object_node:set_x("Pixels", 20)
	object_node:set_width("Absolute", 20.0)
	object_node:set_height("Absolute", 20.0)
	object_node:add_element_index(9)
	node:add_child_index(object_node_index)
	
	--- map 
	_G.node_to_entity_map[button_node_index] = entity_index
	_G.node_to_entity_map[node_index] = entity_index
	entity_to_node_map[entity_index] = node_index
	local is_expanded = expanded_entities[entity_index]
	local display_name = entity.name
	local children = entity.children_indices
	local has_children = #children > 0	

	local render_components = entity.render_component_indices
	local rigid_body_index = entity.rigid_body_index
	local has_render_components = #render_components > 0
	local has_rigid_body = rigid_body_index > -1

	-- quad
	local quad_index = 2
	if darker then quad_index = 3 end
	darker = not darker
	node:add_element_index(quad_index)
	if entity_index == selected_entity then
		selected_node = node_index
		node:add_element_index(8)
	end
	--- expanded/collapsed visual image
	if has_children or has_render_components or has_rigid_body then
		if is_expanded then
			button_node:add_element_index(5)
		else
			button_node:add_element_index(4)
		end
	end

	-- text
	local text_index = get_next_graph_text_index(display_name)
	local text = gui:get_text(text_index)
	text.font_size = 15.0
	text.auto_wrap_distance = 1000.0
	text_node:add_element_index(text_index)
	current_used_text_count = current_used_text_count + 1
	
	graph_height = graph_height + graph_entity_height

	if is_expanded then

		--- transform
		local transform_index = entity.transform_index
		local transform_node_index = get_next_graph_node_index()
		_G.node_to_transform_map[transform_node_index] = transform_index
		local transform_node = gui:get_node(transform_node_index)
		transform_node:reset()
		transform_node:set_x("Pixels", (depth + 1) * graph_child_indent)
		transform_node:set_y("Pixels", graph_height)
		transform_node:set_width("Factor", 1.0)
		transform_node:set_height("Absolute", 20.0)
		transform_node:set_anchor_point(AnchorPoint.TopLeft)
		transform_node:add_element_index(quad_index)
		transform_node:add_hover_action("hover_cursor", 0)
		transform_node:add_left_up_action("open_transform_editor", 3)
		scene_graph_root_node:add_child_index(transform_node_index)
		--- transform icon
		local transform_icon_node_index = get_next_graph_node_index()
		local transform_icon_node = gui:get_node(transform_icon_node_index)
		transform_icon_node:reset()
		transform_icon_node:set_x("Pixels", 0)
		transform_icon_node:set_width("Absolute", 20.0)
		transform_icon_node:set_height("Absolute", 20.0)
		transform_icon_node:add_element_index(6)
		transform_node:add_child_index(transform_icon_node_index)
		--- transform text node
		local transform_text_node_index = get_next_graph_node_index()
		local transform_text_node = gui:get_node(transform_text_node_index)
		transform_text_node:reset()
		transform_text_node:set_x("Pixels", 20)
		transform_text_node:set_width("Factor", 1.0)
		transform_text_node:set_height("Factor", 1.0)
		transform_node:add_child_index(transform_text_node_index)
		--- transform text
		local transform_text_index = get_next_graph_text_index("Transform")
		local transform_text = gui:get_text(transform_text_index)
		transform_text.font_size = 15.0
		transform_text.auto_wrap_distance = 1000.0
		transform_text_node:add_element_index(transform_text_index)
		current_used_text_count = current_used_text_count + 1

		graph_height = graph_height + graph_entity_height
		local render_components = entity.render_component_indices

		--- rigid body
		if has_rigid_body then
			local rigid_body_node_index = get_next_graph_node_index()
			_G.node_to_rigid_body_map[rigid_body_node_index] = rigid_body_index
			local rigid_body_node = gui:get_node(rigid_body_node_index)
			rigid_body_node:reset()
			rigid_body_node:set_x("Pixels", (depth + 1) * graph_child_indent)
			rigid_body_node:set_y("Pixels", graph_height)
			rigid_body_node:set_width("Factor", 1.0)
			rigid_body_node:set_height("Absolute", 20.0)
			rigid_body_node:set_anchor_point(AnchorPoint.TopLeft)
			rigid_body_node:add_element_index(quad_index)
			rigid_body_node:add_hover_action("hover_cursor", 0)
			rigid_body_node:add_left_up_action("open_rigid_body_editor", 3)
			scene_graph_root_node:add_child_index(rigid_body_node_index)
			--- rigid body icon
			local rigid_body_icon_node_index = get_next_graph_node_index()
			local rigid_body_icon_node = gui:get_node(rigid_body_icon_node_index)
			rigid_body_icon_node:reset()
			rigid_body_icon_node:set_x("Pixels", 0)
			rigid_body_icon_node:set_width("Absolute", 20.0)
			rigid_body_icon_node:set_height("Absolute", 20.0)
			rigid_body_icon_node:add_element_index(11)
			rigid_body_node:add_child_index(rigid_body_icon_node_index)
			--- rigid body text node
			local rigid_body_text_node_index = get_next_graph_node_index()
			local rigid_body_text_node = gui:get_node(rigid_body_text_node_index)
			rigid_body_text_node:reset()
			rigid_body_text_node:set_x("Pixels", 20)
			rigid_body_text_node:set_width("Factor", 1.0)
			rigid_body_text_node:set_height("Factor", 1.0)
			rigid_body_node:add_child_index(rigid_body_text_node_index)
			--- rigid body text
			local rigid_body_text_index = get_next_graph_text_index("Rigid Body")
			local rigid_body_text = gui:get_text(rigid_body_text_index)
			rigid_body_text.font_size = 15.0
			rigid_body_text.auto_wrap_distance = 1000.0
			rigid_body_text_node:add_element_index(rigid_body_text_index)
			current_used_text_count = current_used_text_count + 1

			graph_height = graph_height + graph_entity_height
		end
		--- render components
		for i = 1, #render_components do
			local render_component_index = render_components[i]

			local render_component_node_index = get_next_graph_node_index()
			_G.node_to_render_components_map[render_component_node_index] = render_component_index
			local render_component_node = gui:get_node(render_component_node_index)
			render_component_node:reset()
			render_component_node:set_x("Pixels", (depth + 1) * graph_child_indent)
			render_component_node:set_y("Pixels", graph_height)
			render_component_node:set_width("Factor", 1.0)
			render_component_node:set_height("Absolute", 20.0)
			render_component_node:set_anchor_point(AnchorPoint.TopLeft)
			render_component_node:add_element_index(quad_index)
			render_component_node:add_hover_action("hover_cursor", 0)
			render_component_node:add_left_up_action("open_render_component_editor", 3)
			scene_graph_root_node:add_child_index(render_component_node_index)
			--- render component icon
			local render_component_icon_node_index = get_next_graph_node_index()
			local render_component_icon_node = gui:get_node(render_component_icon_node_index)
			render_component_icon_node:reset()
			render_component_icon_node:set_x("Pixels", 0)
			render_component_icon_node:set_width("Absolute", 20.0)
			render_component_icon_node:set_height("Absolute", 20.0)
			render_component_icon_node:add_element_index(7)
			render_component_node:add_child_index(render_component_icon_node_index)
			--- render component text node
			local render_component_text_node_index = get_next_graph_node_index()
			local render_component_text_node = gui:get_node(render_component_text_node_index)
			render_component_text_node:reset()
			render_component_text_node:set_x("Pixels", 20)
			render_component_text_node:set_width("Factor", 1.0)
			render_component_text_node:set_height("Factor", 1.0)
			render_component_node:add_child_index(render_component_text_node_index)
			--- render component text
			local render_component_text_index = get_next_graph_text_index("Render Component")
			local render_component_text = gui:get_text(render_component_text_index)
			render_component_text.font_size = 15.0
			render_component_text.auto_wrap_distance = 1000.0
			render_component_text_node:add_element_index(render_component_text_index)
			current_used_text_count = current_used_text_count + 1

			graph_height = graph_height + graph_entity_height
		end	
		-- recursively process children
		for i = 1, #children do
			local child_entity_index = children[i]
			local child_entity = Engine.scene:get_entity(child_entity_index)
			build_graph_recursive(child_entity, child_entity_index, depth + 1)
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

function resize_horizontal_from_right()
	resize_called_this_tick = true

	local window_size = Engine.client.window_size
	local parent = gui.ActiveNode:get_parent()
	local original_width = parent.size.x
	local original_min_world_space = parent.position.x
	local delta = Engine.client.cursor_position.x - original_min_world_space
	parent:set_width("Absolute", original_width - delta)
end

function height_to_factor()
	local parent = gui.ActiveNode:get_parent()

	local grand_parent = parent:get_parent();
	
	parent:set_height("Factor", parent.size.y / grand_parent.size.y)
end

function width_to_factor()
	local parent = gui.ActiveNode:get_parent()

	local grand_parent = parent:get_parent();
	
	parent:set_width("Factor", parent.size.x / grand_parent.size.x)
end

function resize_vertical()
	resize_called_this_tick = true

	local window_size = Engine.client.window_size
	local parent = gui.ActiveNode:get_parent()
	local original_height = parent.size.y
	local original_max_world_space = parent.position.y + original_height
	local delta = Engine.client.cursor_position.y - original_max_world_space
	parent:set_height("Absolute", original_height + delta)
end

function toggle_running()
	Engine.scene.running = not Engine.scene.running
	play_node.hidden = not play_node.hidden
	pause_node.hidden = not pause_node.hidden
end
function step()
	if not Engine.scene.running then
		Engine.scene:step(100 / 1000)		
	end
end
function recompile() 
    Engine.client.flags.reload_rendering_queued = true
    Engine.client.flags.reload_scripts_queued = true
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