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

function Update()
	local gui = Engine.renderer.gui
	local scene_graph_node = gui:get_node(3)
	scene_graph_node.position = scene_graph_node.position + Vector.new2(0.0001, 0.0) 
end