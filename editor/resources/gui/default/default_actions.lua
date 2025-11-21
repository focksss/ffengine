---@meta
---@type userdata Engine

---@type number dt

function color_quad_bright()
	Engine.renderer.GUI.ActiveNode.quad:set_color(0.7, 0.7, 0.7, 1.0)
end

function color_quad_bright1()
	Engine.renderer.GUI.ActiveNode.quad:set_color(0.4, 0.4, 0.4, 1.0)
end

function color_quad_normal()
	Engine.renderer.GUI.ActiveNode.quad:set_color(0.5, 0.5, 0.5, 1.0)
end

function color_quad_normal1()
	Engine.renderer.GUI.ActiveNode.quad:set_color(0.3, 0.3, 0.3, 1.0)
end

function reload_gui()
    Engine.controller:set_reload_gui(true)
end

function reload_shaders()
    Engine.controller:set_reload_shaders(true)
end

function screenshot()
    Engine.controller:set_screenshot(true)
end

local fps_counter = 0
local time_since_fps_update = 0
function update_fps_display()
    if time_since_fps_update > 1.0 then
        local fps = fps_counter / time_since_fps_update
        Engine.renderer.GUI.ActiveNode.text:update_text(string.format("FPS: %.1f", fps))
        time_since_fps_update = 0
        fps_counter = 0
    end

    time_since_fps_update = time_since_fps_update + dt
    fps_counter = fps_counter + 1
end

local time_since_position_update = 0
function update_position_display()
    if time_since_position_update > 0.1 then
    	local x, y, z = Engine.controller:get_camera_position()
    	Engine.renderer.GUI.ActiveNode.text:update_text(string.format("Cam pos: X: %.2f, Y: %.2f, Z: %.2f", x, y, z))
    	time_since_position_update = 0
    end

    time_since_position_update = time_since_position_update + dt
end

function toggle_hitbox_view()
    Engine.controller:toggle_draw_hitboxes()
end

function toggle_physics_tick()
    Engine.controller:toggle_physics()
end

function toggle_player_physics()
    Engine.controller:toggle_player_physics()
end

function toggle_text()
    local current_text = Engine.renderer.GUI.ActiveNode.text.text_message
    if current_text == "On" then
    	Engine.renderer.GUI.ActiveNode.text:update_text("Off")
    else
    	Engine.renderer.GUI.ActiveNode.text:update_text("On")
    end
end