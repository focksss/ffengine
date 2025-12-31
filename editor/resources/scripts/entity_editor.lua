_G.selected_transform = 0
_G.selected_rigid_body = 0

local gui

local entity_editor_area_node
    local entity_editor_root_node
        local editor_height = 0
    local entity_editor_scroll_bar_node

local transform_editor_ui_node
local render_component_editor_ui_node
local rigid_body_editor_ui_node

function Awake()

	gui = Engine.renderer:gui(0)

    local root_node = gui:get_root(0)
    local right_area_node = root_node:get_child(1)
        entity_editor_area_node = right_area_node:get_child(1)
            entity_editor_root_node = entity_editor_area_node:get_child(0)
            entity_editor_scroll_bar_node = entity_editor_area_node:get_child(1)

	transform_editor_ui_node = gui:get_unparented(2)
	render_component_editor_ui_node = gui:get_unparented(3)
	rigid_body_editor_ui_node = gui:get_unparented(4)

end

local queued_entity_open_from_read = 0
function Update()
    if gui == nil then return end

    if queued_entity_open_from_read > 0 then
        queued_entity_open_from_read = queued_entity_open_from_read + 1
    end
    if queued_entity_open_from_read > 1 then
        Engine.scene:reset_outlines()
        local hovered_entity_index = Engine.renderer.scene_renderer.hovered_entity - 1
        if hovered_entity_index > -1 then
            _G.Editor.select_entity(hovered_entity_index)
            local selected_render_component = Engine.renderer.scene_renderer.hovered_child_component_number - 1
            render_component_editor(Engine.scene:get_entity(hovered_entity_index):get_render_component_index(selected_render_component))
            queued_entity_open_from_read = 0
        else
            _G.Editor.deselect() 
        end
    end

    entity_editor_scroll_bar_node:set_height("Factor",
		math.min(1.0, entity_editor_area_node.size.y / editor_height) --- visible pixels / total pixels
	)
end
function click_entity()
    Engine.renderer.scene_renderer.queue_hovered_component_read = true --- TEMP
    queued_entity_open_from_read = 1
end

_G.editor_node_to_render_component_map = {}
local owned_render_component_editor_node_indices = {}
local current_used_render_component_editor_node_count = 0
local function get_next_render_component_editor_node_index()
	local current_owned_node_count = #owned_render_component_editor_node_indices
	if current_used_render_component_editor_node_count >= current_owned_node_count then
		owned_render_component_editor_node_indices[current_owned_node_count + 1] = gui:clone_node(render_component_editor_ui_node.index, entity_editor_root_node.index)
	end
	current_used_render_component_editor_node_count = current_used_render_component_editor_node_count + 1
	return owned_render_component_editor_node_indices[current_used_render_component_editor_node_count]
end

function open_entity_editor()
    Engine.scene:reset_outlines()
    entity_editor_root_node:clear_children()
    
    --- reset mappings and counters
    _G.editor_node_to_render_component_map = {}
    current_used_render_component_editor_node_count = 0

    local entity = Engine.scene:get_entity(_G.node_to_entity_map[gui.ActiveNode.index])

    --- add transform editor
    _G.selected_transform = entity.transform_index
	entity_editor_root_node:add_child_index(transform_editor_ui_node.index)

    --- add rigid body editor
    if entity.rigid_body_index > -1 then
        _G.selected_rigid_body = entity.rigid_body_index
    	entity_editor_root_node:add_child_index(rigid_body_editor_ui_node.index)        
    end

    outline_entity(entity.index)

    --- add render component editors
    local render_component_indices = entity.render_component_indices
    for i = 1, #render_component_indices do
        local render_component_index = render_component_indices[i]

        local render_component_editor_node_index = get_next_render_component_editor_node_index()
        local render_component_editor_node = gui:get_node(render_component_editor_node_index)
        _G.editor_node_to_render_component_map[render_component_editor_node_index] = render_component_index
        _G.editor_node_to_render_component_map[render_component_editor_node:get_child_index(0)] = render_component_index --- map expansion toggle
        entity_editor_root_node:add_child_index(render_component_editor_node_index)
    end

end
function outline_entity(index)
    local entity = Engine.scene:get_entity(index)

    local render_component_indices = entity.render_component_indices
    for i = 1, #render_component_indices do
        local render_component_index = render_component_indices[i]
        Engine.scene:add_outlined_component(render_component_index);
    end

    local rigid_body_index = entity.rigid_body_index
    if rigid_body_index > -1 then
        Engine.scene:add_outlined_body(rigid_body_index)
    end

    local children_indices = entity.children_indices
    for i = 1, #children_indices do
        outline_entity(children_indices[i])
    end
end

function open_transform_editor() 
	_G.selected_transform = _G.node_to_transform_map[gui.ActiveNode.index]
    _G.Editor.select_entity(Engine.scene:get_transform(_G.selected_transform).owner_index)

	entity_editor_root_node:clear_children()
	entity_editor_root_node:add_child_index(transform_editor_ui_node.index)
end
function open_render_component_editor() 
    render_component_editor(_G.node_to_render_components_map[gui.ActiveNode.index])
end
function open_rigid_body_editor() 
	_G.selected_rigid_body = _G.node_to_rigid_body_map[gui.ActiveNode.index]
    _G.Editor.select_entity(Engine.scene:get_rigid_body(_G.selected_rigid_body).owner_index)

	entity_editor_root_node:clear_children()
	entity_editor_root_node:add_child_index(rigid_body_editor_ui_node.index)
end

function render_component_editor(component_index) 

	local selected_render_component = Engine.scene:get_render_component(component_index).index
    Engine.scene:add_outlined_component(selected_render_component);

    entity_editor_root_node:clear_children()
    
    --- reset mappings and counters
    _G.editor_node_to_render_component_map = {}
    current_used_render_component_editor_node_count = 0

    --- add render component editor
    local render_component_editor_node_index = get_next_render_component_editor_node_index()
    local render_component_editor_node = gui:get_node(render_component_editor_node_index)
    _G.editor_node_to_render_component_map[render_component_editor_node_index] = selected_render_component
    _G.editor_node_to_render_component_map[render_component_editor_node:get_child_index(0)] = selected_render_component --- map expansion toggle
    entity_editor_root_node:add_child_index(render_component_editor_node_index)

end