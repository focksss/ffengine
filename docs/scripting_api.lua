---@meta

---@type EngineClass
Engine

---@type number
dt = dt

---@class MovementMode
---@field GHOST integer
---@field PHYSICS integer
---@field EDITOR integer
MovementMode = MovementMode

MovementMode = {
    GHOST = 0,
    PHYSICS = 1,
    EDITOR = 2,
}

---@class Vector
---@field x number
---@field y number
---@field z number
---@field w number

---@field new fun(x:number, y:number, z:number, w:number):Vector

Vector = Vector

---@class EngineClass
---@field renderer Renderer
---@field controller Controller

    ---@class Renderer
    ---@field gui GUI

        ---@class GUI
        ---@field ActiveNode GUINode

            ---@class GUINode
            ---@field quad GUIQuad
            ---@field text GUIText

                ---@class GUIQuad
                ---@field set_color fun(self:GUIQuad, r:number, g:number, b:number, a:number):void

                ---@class GUIText
                ---@field text_message string
                ---@field update_text fun(self:GUIText, text:string):void

    ---@class Controller
    ---@field flags Flags
    ---@field player Player

        ---@class Flags
        ---@field reload_gui_queued boolean
        ---@field reload_shaders_queued boolean
        ---@field pause_rendering boolean
        ---@field screenshot_queued boolean
        ---@field draw_hitboxes boolean
        ---@field do_physics boolean

    ---@class Player
    ---@field movement_mode integer
    ---@field rigid_body RigidBody

        ---@class RigidBody
        ---@field position Vector

