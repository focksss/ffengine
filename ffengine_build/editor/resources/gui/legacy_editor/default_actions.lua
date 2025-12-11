local fps_counter = 0
local time_since_fps_update = 0.0
local time_since_position_update = 0.0
local gui_path = "editor\\resources\\gui\\default\\default.gui"
local gui_reload_queued = false
function Update()
    if gui_reload_queued then 
        Engine.renderer.gui:load_from_file("editor\\resources\\gui\\default\\default.gui")
        gui_reload_queued = false
    end

    time_since_fps_update = time_since_fps_update + dt
    fps_counter = fps_counter + 1

    time_since_position_update = time_since_position_update + dt
end

function color_quad_bright()
	Engine.renderer.gui.ActiveNode.quad:set_color(0.7, 0.7, 0.7, 1.0)
end

function color_quad_bright1()
	Engine.renderer.gui.ActiveNode.quad:set_color(0.4, 0.4, 0.4, 1.0)
end

function color_quad_normal()
	Engine.renderer.gui.ActiveNode.quad:set_color(0.5, 0.5, 0.5, 1.0)
end

function color_quad_normal1()
	Engine.renderer.gui.ActiveNode.quad:set_color(0.3, 0.3, 0.3, 1.0)
end

function reload_gui()
    Engine.controller.flags.reload_gui_queued = true
    ---gui_reload_queued = true
end

function reload_shaders()
    Engine.controller.flags.reload_shaders_queued = true
end

function screenshot()
    Engine.controller.flags.screenshot_queued = true
end

function update_fps_display()
    if time_since_fps_update > 1.0 then
        local fps = fps_counter / time_since_fps_update
        Engine.renderer.gui.ActiveNode.text:update_text(string.format("FPS: %.1f", fps))
        time_since_fps_update = 0
        fps_counter = 0
    end
end

function update_position_display()
    local player = Engine.physics_engine:get_player(0);
    if time_since_position_update > 0.1 then
    	local x = player.rigid_body.position
    	Engine.renderer.gui.ActiveNode.text:update_text(string.format("Cam pos: X: %.2f, Y: %.2f, Z: %.2f", x.x, x.y, x.z))
    	time_since_position_update = 0
    end
end

function toggle_hitbox_view()
    Engine.controller.flags.draw_hitboxes = not Engine.controller.flags.draw_hitboxes
end

function toggle_physics_tick()
    Engine.controller.flags.do_physics = not Engine.controller.flags.do_physics
end

function toggle_player_physics()
    local player = Engine.physics_engine:get_player(0);

    if player.movement_mode == MovementMode.GHOST then
        player.movement_mode = MovementMode.EDITOR
        Engine.renderer.gui.ActiveNode.text:update_text("Editor")
    elseif player.movement_mode == MovementMode.EDITOR then
        player.movement_mode = MovementMode.PHYSICS
        Engine.renderer.gui.ActiveNode.text:update_text("Physics")
    else
        player.movement_mode = MovementMode.GHOST
        Engine.renderer.gui.ActiveNode.text:update_text("Ghost")
    end
end

function toggle_text()
    local current_text = Engine.renderer.gui.ActiveNode.text.text_message
    if current_text == "On" then
    	Engine.renderer.gui.ActiveNode.text:update_text("Off")
    else
    	Engine.renderer.gui.ActiveNode.text:update_text("On")
    end
end

function reload_all_scripts()
    Engine.controller.flags.reload_all_scripts_queued = true
end