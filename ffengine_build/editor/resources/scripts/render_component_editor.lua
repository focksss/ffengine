local gui

function Awake()
    gui = Engine.renderer:gui(0)
end

local expanded_render_components = {}

function collapse()
    local component_index = _G.editor_node_to_render_component_map[gui.ActiveNode.index]
    if expanded_render_components[component_index] == nil then expanded_render_components[component_index] = true end
    local collapsed = expanded_render_components[component_index]
    if not collapsed then
        expanded_render_components[component_index] = true
        gui.ActiveNode:get_parent():set_height("Absolute", 20.0)
    else
        expanded_render_components[component_index] = false
        gui.ActiveNode:get_parent():set_height("Absolute", 215.0)
    end
end