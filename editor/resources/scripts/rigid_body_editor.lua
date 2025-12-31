local gui

function Awake()
    gui = Engine.renderer:gui(0)
end

local time_since_text_update = 0
function update_static_display()
	time_since_text_update = time_since_text_update + dt

	if time_since_text_update > 0.1 then
        local text = "False" 
        if Engine.scene:get_rigid_body(_G.selected_rigid_body).static then text = "True" end
        gui.ActiveNode:get_text_at(0):update_text(text)
        time_since_text_update = 0
	end
end

function flip_static()
    local rigid_body = Engine.scene:get_rigid_body(_G.selected_rigid_body)
    rigid_body.static = not rigid_body.static
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