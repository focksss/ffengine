local fly_speed = 1.0
local sense = 0.001

function Update()
    local player = Engine.physics_engine:get_player(0);
    local camera = player.camera
    local rigid_body = player.rigid_body

    local window_size = Engine.controller.window_size
    camera.aspect_ratio = window_size.x / window_size.y

    if Engine.controller:new_key_pressed(KeyCode.Escape) then
        Engine.controller.cursor_locked = not Engine.controller.cursor_locked
    end

    local move_direction = Vector.new(0.0, 0.0, 0.0, 0.0)
    local rot = camera.rotation
    if Engine.controller:key_pressed(KeyCode.KeyW) then
        move_direction.x = move_direction.x + math.cos(rot.y + math.pi * 0.5)
        move_direction.z = move_direction.z - math.sin(rot.y + math.pi * 0.5)
    end
    if Engine.controller:key_pressed(KeyCode.KeyA) then
        move_direction.x = move_direction.x - math.cos(rot.y)
        move_direction.z = move_direction.z + math.sin(rot.y)
    end
    if Engine.controller:key_pressed(KeyCode.KeyS) then
        move_direction.x = move_direction.x - math.cos(rot.y + math.pi * 0.5)
        move_direction.z = move_direction.z + math.sin(rot.y + math.pi * 0.5)
    end
    if Engine.controller:key_pressed(KeyCode.KeyD) then
        move_direction.x = move_direction.x + math.cos(rot.y)
        move_direction.z = move_direction.z - math.sin(rot.y)
    end

    if Engine.controller:key_pressed(KeyCode.Space) then
        if player.movement_mode == MovementMode.PHYSICS then
            if player.grounded then
                move_direction.y = move_direction.y + 1.0
            end
        else
            move_direction.y = move_direction.y + 1.0
        end
    end

    if player.movement_mode == MovementMode.GHOST then
        rigid_body.velocity = Vector.new(0.0, 0.0, 0.0, 0.0)

        if Engine.controller:key_pressed(KeyCode.ShiftLeft) then
            move_direction.y = move_direction.y - 1.0
        end

        rigid_body.position = rigid_body.position + (move_direction:normalize3() * fly_speed * dt)
    elseif player.movement_mode == MovementMode.PHYSICS then
        local dimensional_speed = Vector.new(1.0, 3.0, 1.0, 0.0)

        rigid_body.velocity = rigid_body.velocity + (move_direction * dimensional_speed * dt)
    end

    if rigid_body.position.y < -20.0 then
        rigid_body.position = Vector.new(0.0, 10.0, 0.0, 0.0)
        rigid_body.velocity = Vector.new(0.0, 0.0, 0.0, 0.0)
    end
end

function MouseScrolled()
    fly_speed = fly_speed + 0.1 * Engine.controller.scroll_delta.y
end

function MouseMoved()
    local player = Engine.physics_engine:get_player(0);
    local camera = player.camera

    local new_rot = Vector.new(0.0, 0.0, 0.0, 0.0)

    local rot_x_delta = Engine.controller.mouse_delta.y
    local rot_y_delta = Engine.controller.mouse_delta.x

    new_rot.y = camera.rotation.y + rot_y_delta * sense
    new_rot.x = camera.rotation.x - rot_x_delta * sense
    new_rot.x = math.max(-math.pi * 0.5, math.min(new_rot.x, math.pi * 0.5))

    camera.rotation = new_rot
end