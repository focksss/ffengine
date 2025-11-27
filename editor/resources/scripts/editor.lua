local initial_mouse_pos = Vector.new()
local initial_value = Vector.new()

local resize_called_last_tick = false
local resize_called_this_tick = false

function close_window() 
    Engine.client.flags.close_requested = true
end

function recompile() 
    Engine.client.flags.recompile_queued = true
end

function close_hovered()
	Engine.renderer.gui.ActiveNode.quad:set_color(1.0, 0.3, 0.3, 1.0)
end

function color_hovered()
	Engine.renderer.gui.ActiveNode.quad:set_color(0.3, 0.3, 0.3, 1.0)
end

function color_unhovered()
	Engine.renderer.gui.ActiveNode.quad:set_color(0.0, 0.0, 0.0, 0.0)
end

function resize_scene_graph()
	resize_called_this_tick = true

	local gui = Engine.renderer.gui
	local scene_graph_node = gui:get_node(3)
	local window_size = Engine.client.window_size
	scene_graph_node.scale = Vector.new2(
		window_size.x 
		- Engine.client.cursor_position.x
		+ 5, scene_graph_node.scale.y
	) 
	
	local scene_viewport = Engine.renderer.scene_renderer.viewport
	scene_viewport.width = window_size.x - scene_graph_node.scale.x
end

function Update()
	if resize_called_last_tick and not resize_called_this_tick then
		Engine.client.flags.recompile_queued = true
	end
	
	resize_called_last_tick = resize_called_this_tick
	resize_called_this_tick = false

	local gui = Engine.renderer.gui
	local scene_graph_node = gui:get_node(3)
	scene_graph_node.scale = Vector.new2(scene_graph_node.scale.x, Engine.client.window_size.y - 40)
end