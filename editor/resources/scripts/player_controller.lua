local fly_speed = 1.0
local sense = 0.001
editor_cam_move_sense = 0.002
editor_cam_rot_sense = 0.005
--- editor camera vars
editor_target = Vector.new()
editor_rotation = Vector.new()
editor_distance = 5.0
--- for tracking mouse when moving editor camera with middle click
initial_mouse_pos = Vector.new()
initial_value = Vector.new()
--- false signifies changing position (for editor camera)
changing_rotation = false

camera_index = 0
transform_index = 0
camera = nil
transform = nil

function Awake() 
    camera = Engine.scene:get_camera(camera_index)
    transform_index = camera.transform
    transform = Engine.scene:get_transform(transform_index)
end

function Update()
    if camera == nil then return end

    local scene_viewport = Engine.renderer.scene_renderer.viewport
	camera.aspect_ratio = scene_viewport.width / scene_viewport.height

    if Engine.client:new_key_pressed(KeyCode.Escape) then
        Engine.client.cursor_locked = not Engine.client.cursor_locked
    end

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
    transform.translation = editor_target + (Vector.new3(0.0, 0.0, 1.0):rotate_by_euler(editor_rotation) * editor_distance)
    transform.rotation = (editor_rotation):euler_to_quat()
end
    --[[
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
    --]]
function MouseScrolled()
    if Engine.renderer:gui(0):is_node_hovered(Engine.renderer:gui(0):get_root(0):get_child(2).index) then
        local zoom_factor = 1.0 - (Engine.client.scroll_delta.y * 0.1)
        editor_distance = editor_distance * zoom_factor
        editor_distance = math.max(0.01, editor_distance)
    end
end

function MouseButtonPressed()
    if (Engine.client.ButtonPressed == MouseButton.Middle) then
        initial_mouse_pos = Engine.client.cursor_position
        if Engine.client:key_pressed(KeyCode.ShiftLeft) then
            initial_value = editor_target
            changing_rotation = false
        else
            initial_value = editor_rotation
            changing_rotation = true
        end
    end
end