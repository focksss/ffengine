function close_window() 
    Engine.controller.flags.close_requested = true
end

function recompile() 
    Engine.controller.flags.recompile_queued = true
end

function close_hovered()
	Engine.renderer.gui.ActiveNode.quad:set_color(1.0, 0.3, 0.3, 1.0)
end

function close_unhovered()
	Engine.renderer.gui.ActiveNode.quad:set_color(0.1, 0.1, 0.1, 0.9)
end

function color_hovered()
	Engine.renderer.gui.ActiveNode.quad:set_color(0.5, 0.5, 0.5, 0.9)
end

function color_unhovered()
	Engine.renderer.gui.ActiveNode.quad:set_color(0.3, 0.3, 0.3, 0.9)
end