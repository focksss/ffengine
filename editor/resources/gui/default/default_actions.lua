function color_quad_bright()
	GUI:get_quad(GUI:get_node(GUI.ActiveNode).quad):set_color(0.7, 0.7, 0.7, 1.0)
end

function color_quad_bright1()
	GUI:get_quad(GUI:get_node(GUI.ActiveNode).quad):set_color(0.4, 0.4, 0.4, 1.0)
end

function color_quad_normal()
	GUI:get_quad(GUI:get_node(GUI.ActiveNode).quad):set_color(0.5, 0.5, 0.5, 1.0)
end

function color_quad_normal1()
	GUI:get_quad(GUI:get_node(GUI.ActiveNode).quad):set_color(0.3, 0.3, 0.3, 1.0)
end

function reload_gui()
    controller:set_reload_gui(true)
end

function reload_shaders()
    controller:set_reload_shaders(true)
end

function screenshot()
    controller:set_screenshot(true)
end

local fps_counter = 0
local time_since_fps_update = 0
function update_fps_display()
    if time_since_fps_update > 1.0 then
        local fps = fps_counter / time_since_fps_update
        GUI:update_text_of_node(GUI.ActiveNode, string.format("FPS: %.1f", fps))
        time_since_fps_update = 0
        fps_counter = 0
    end

    time_since_fps_update = time_since_fps_update + dt
    fps_counter = fps_counter + 1
end

local time_since_position_update = 0
function update_position_display()
    if time_since_position_update > 0.1 then
    	local x, y, z = controller:get_camera_position()
    	GUI:update_text_of_node(GUI.ActiveNode, string.format("Cam pos: X: %.2f, Y: %.2f, Z: %.2f", x, y, z))
    	time_since_position_update = 0
    end

    time_since_position_update = time_since_position_update + dt
end

function toggle_hitbox_view()
    controller:toggle_draw_hitboxes()
end

function toggle_physics_tick()
    controller:toggle_physics()
end

function toggle_player_physics()
    controller:toggle_player_physics()
end

function toggle_text()
    local current_text = GUI:get_text(GUI:get_node(GUI.ActiveNode).text).text_message
    if current_text == "On" then
    	GUI:update_text_of_node(GUI.ActiveNode, "Off")
    else
    	GUI:update_text_of_node(GUI.ActiveNode, "On")
    end
end