local fly_speed = 1.0
local sense = 0.001
local editor_cam_move_sense = 0.002
local editor_cam_rot_sense = 0.005
--- editor camera vars
local editor_target = Vector.new()
local editor_rotation = Vector.new()
local editor_distance = 5.0
--- for tracking mouse when moving editor camera with middle click
local initial_mouse_pos = Vector.new()
local initial_value = Vector.new()
--- false signifies changing position (for editor camera)
local changing_rotation = false

function Update()
    local player = Engine.physics_engine:get_player(0);
    local camera = player.camera
    local rigid_body = player.rigid_body

    local scene_viewport = Engine.renderer.scene_renderer.viewport
	camera.aspect_ratio = scene_viewport.width / scene_viewport.height

    if Engine.client:new_key_pressed(KeyCode.Escape) then
        Engine.client.cursor_locked = not Engine.client.cursor_locked
    end

    if player.movement_mode ~= MovementMode.EDITOR then
        local move_direction = Vector.new()
        local rot = camera.rotation
        if Engine.client:key_pressed(KeyCode.KeyW) then
            move_direction.x = move_direction.x + math.cos(rot.y + math.pi * 0.5)
            move_direction.z = move_direction.z - math.sin(rot.y + math.pi * 0.5)
        end
        if Engine.client:key_pressed(KeyCode.KeyA) then
            move_direction.x = move_direction.x - math.cos(rot.y)
            move_direction.z = move_direction.z + math.sin(rot.y)
        end
        if Engine.client:key_pressed(KeyCode.KeyS) then
            move_direction.x = move_direction.x - math.cos(rot.y + math.pi * 0.5)
            move_direction.z = move_direction.z + math.sin(rot.y + math.pi * 0.5)
        end
        if Engine.client:key_pressed(KeyCode.KeyD) then
            move_direction.x = move_direction.x + math.cos(rot.y)
            move_direction.z = move_direction.z - math.sin(rot.y)
        end

        if Engine.client:key_pressed(KeyCode.Space) then
            if player.movement_mode == MovementMode.PHYSICS then
                if player.grounded then
                    move_direction.y = move_direction.y + 1.0
                end
            else
                move_direction.y = move_direction.y + 1.0
            end
        end

        if player.movement_mode == MovementMode.GHOST then
            rigid_body.velocity = Vector.new()

            if Engine.client:key_pressed(KeyCode.ShiftLeft) then
                move_direction.y = move_direction.y - 1.0
            end

            rigid_body.position = rigid_body.position + (move_direction:normalize3() * fly_speed * dt)
        elseif player.movement_mode == MovementMode.PHYSICS then
            local dimensional_speed = Vector.new3(25.0, 1000.0, 25.0)

            rigid_body.velocity = rigid_body.velocity + (move_direction:normalize3() * dimensional_speed * dt)
        end

        if rigid_body.position.y < -20.0 then
            rigid_body.position = Vector.new3(0.0, 10.0, 0.0)
            rigid_body.velocity = Vector.new()
        end

        camera.position = rigid_body.position
    else ---editor camera mode this tests
        if Engine.client:mouse_button_pressed(MouseButton.Middle) then
            local delta_pixels = Engine.client.cursor_position - initial_mouse_pos;

            if changing_rotation then
                editor_rotation = initial_value - Vector.new2(delta_pixels.y, delta_pixels.x) * editor_cam_rot_sense
            else
                local horizontal_axis = Vector.new3(1.0, 0.0, 0.0):rotate_by_euler(editor_rotation)
                local vertical_axis = Vector.new3(0.0, 1.0, 0.0):rotate_by_euler(editor_rotation)
                local horiz_add = horizontal_axis * delta_pixels.x * editor_distance * editor_cam_move_sense * -1.0
                local vert_add = vertical_axis * delta_pixels.y * editor_distance * editor_cam_move_sense
                editor_target = initial_value + horiz_add + vert_add
            end
        end

        editor_rotation.z = 0.0
        rigid_body.velocity = Vector.new()
        camera.position = editor_target + (Vector.new3(0.0, 0.0, 1.0):rotate_by_euler(editor_rotation) * editor_distance)
        camera.rotation = editor_rotation
    end
end

function MouseScrolled()
    local player = Engine.physics_engine:get_player(0);
    if player.movement_mode == MovementMode.GHOST then
        fly_speed = fly_speed * math.pow(1.1, Engine.client.scroll_delta.y)
                fly_speed = math.max(0.01, fly_speed)
    elseif player.movement_mode == MovementMode.EDITOR then
        local zoom_factor = 1.0 - (Engine.client.scroll_delta.y * 0.1)
        editor_distance = editor_distance * zoom_factor
        editor_distance = math.max(0.01, editor_distance)
    end
end

function MouseMoved()
    local player = Engine.physics_engine:get_player(0);
    local camera = player.camera

    local new_rot = Vector.new()

    local rot_x_delta = Engine.client.mouse_delta.y
    local rot_y_delta = Engine.client.mouse_delta.x

    new_rot.y = camera.rotation.y + rot_y_delta * sense
    new_rot.x = camera.rotation.x - rot_x_delta * sense
    new_rot.x = math.max(-math.pi * 0.5, math.min(new_rot.x, math.pi * 0.5))

    camera.rotation = new_rot
end

function MouseButtonPressed()
    local player = Engine.physics_engine:get_player(0);
    if (Engine.client.ButtonPressed == MouseButton.Middle) and (player.movement_mode == MovementMode.EDITOR) then
        initial_mouse_pos = Engine.client.cursor_position
        if Engine.client:key_pressed(KeyCode.ShiftLeft) then
            initial_value = editor_target
            changing_rotation = false
        else
            initial_value = player.camera.rotation
            changing_rotation = true
        end
    end
end