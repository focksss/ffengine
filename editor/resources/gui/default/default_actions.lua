function color_quad_bright()
	gui:set_quad_color(node_index, 0.7, 0.7, 0.7, 1.0)
end

function color_quad_bright1()
    gui:set_quad_color(node_index, 0.4, 0.4, 0.4, 1.0)
end

function color_quad_normal()
    gui:set_quad_color(node_index, 0.5, 0.5, 0.5, 1.0)
end

function color_quad_normal1()
    gui:set_quad_color(node_index, 0.3, 0.3, 0.3, 1.0)
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

function update_fps_display()
    if gui:get_storage_elapsed(node_index) > 1.0 then
        local fps = gui:get_storage_value1(node_index) / gui:get_storage_elapsed(node_index)
        gui:update_text_of_node(node_index, string.format("FPS: %.1f", fps))
        gui:reset_storage_time(node_index)
        gui:set_storage_value1(node_index, 0.0)
    end

    local current = gui:get_storage_value1(node_index)
    gui:set_storage_value1(node_index, current + 1.0)
end

function update_position_display()
    if gui:get_storage_elapsed(node_index) > 0.1 then
    	local x, y, z = controller:get_camera_position()
    	gui:update_text_of_node(node_index, string.format("Cam pos: X: %.2f, Y: %.2f, Z: %.2f", x, y, z))
    	gui:reset_storage_time(node_index)
    end
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
    local current_text = gui:get_node_text(node_index)
    if current_text == "On" then
    	gui:update_text_of_node(node_index, "Off")
    else
    	gui:update_text_of_node(node_index, "On")
    end
end