---@meta

---@type EngineClass
Engine = nil

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

---@class EngineClass
---@field renderer Renderer
---@field controller Controller
---@field physics_engine PhysicsEngine

    ---@class Renderer
    ---@field gui GUI

        ---@class GUI
        ---@field ActiveNode GUINode
        ---@field load_from_file fun(self:GUI, path:string):nil

            ---@class GUINode
            ---@field quad GUIQuad
            ---@field text GUIText

                ---@class GUIQuad
                ---@field set_color fun(self:GUIQuad, r:number, g:number, b:number, a:number):nil

                ---@class GUIText
                ---@field text_message string
                ---@field update_text fun(self:GUIText, text:string):nil

    ---@class Controller
    ---@field flags Flags
    ---@field cursor_position Vector
    ---@field scroll_delta Vector
    ---@field mouse_delta Vector
    ---@field cursor_locked boolean
    ---@field window_size Vector
    ---@field ButtonPressed MouseButton
    ---@field ButtonReleased MouseButton
    ---@field new_key_pressed fun(self:Controller, key: integer):boolean
    ---@field key_pressed fun(self:Controller, key: integer):boolean
    ---@field mouse_button_pressed fun(self:Controller, button: integer):boolean

        ---@class Flags
        ---@field reload_gui_queued boolean
        ---@field reload_shaders_queued boolean
        ---@field pause_rendering boolean
        ---@field screenshot_queued boolean
        ---@field draw_hitboxes boolean
        ---@field do_physics boolean
        ---@field reload_all_scripts_queued boolean
        ---@field close_requested boolean

    ---@class PhysicsEngine
    ---@field gravity Vector
    ---@field get_player fun(self:PhysicsEngine, index:integer):Player
    ---@field get_rigid_body fun(self:PhysicsEngine, index:integer):RigidBody

    ---@class Player
    ---@field movement_mode integer
    ---@field grounded boolean
    ---@field rigid_body RigidBody
    ---@field camera Camera

        ---@class RigidBody
        ---@field position Vector
        ---@field velocity Vector

        ---@class Camera
        ---@field position Vector
        ---@field rotation Vector
        ---@field fov_y number
        ---@field aspect_ratio number
        ---@field near number
        ---@field far number



---@class Vector
---@field x number
---@field y number
---@field z number
---@field w number
---@field new_vec4 fun(x:number, y:number, z:number, w:number):Vector
---@field new_vec3 fun(x:number, y:number, z:number):Vector
---@field new_vec2 fun(x:number, y:number):Vector
---@field new_vec fun(x:number):Vector
---@field new fun():Vector
---@field new_empty_quat fun():Vector
---@field normalize_3d fun(self:Vector):Vector
---@field rotate_by_euler fun(self:Vector, rot:Vector):Vector

Vector = Vector


---@class MouseButton
---@field Left integer
---@field Right integer
---@field Middle integer
---@field Back integer
---@field Forward integer
MouseButton = MouseButton

MouseButton = {
    Left = 0,
    Right = 1,
    Middle = 2,
    Back = 3,
    Forard = 4,
}


---@class KeyCode
---@field Backquote integer
---@field Section integer
---@field Backslash integer
---@field BracketLeft integer
---@field BracketRight integer
---@field Comma integer
---@field Digit0 integer
---@field Digit1 integer
---@field Digit2 integer
---@field Digit3 integer
---@field Digit4 integer
---@field Digit5 integer
---@field Digit6 integer
---@field Digit7 integer
---@field Digit8 integer
---@field Digit9 integer
---@field Equal integer
---@field IntlBackslash integer
---@field IntlRo integer
---@field IntlYen integer
---@field KeyA integer
---@field KeyB integer
---@field KeyC integer
---@field KeyD integer
---@field KeyE integer
---@field KeyF integer
---@field KeyG integer
---@field KeyH integer
---@field KeyI integer
---@field KeyJ integer
---@field KeyK integer
---@field KeyL integer
---@field KeyM integer
---@field KeyN integer
---@field KeyO integer
---@field KeyP integer
---@field KeyQ integer
---@field KeyR integer
---@field KeyS integer
---@field KeyT integer
---@field KeyU integer
---@field KeyV integer
---@field KeyW integer
---@field KeyX integer
---@field KeyY integer
---@field KeyZ integer
---@field Minus integer
---@field Period integer
---@field Quote integer
---@field Semicolon integer
---@field Slash integer
---@field AltLeft integer
---@field AltRight integer
---@field Backspace integer
---@field CapsLock integer
---@field ContextMenu integer
---@field ControlLeft integer
---@field ControlRight integer
---@field Enter integer
---@field SuperLeft integer
---@field SuperRight integer
---@field ShiftLeft integer
---@field ShiftRight integer
---@field Space integer
---@field Tab integer
---@field Convert integer
---@field KanaMode integer
---@field Lang1 integer
---@field Lang2 integer
---@field Lang3 integer
---@field Lang4 integer
---@field Lang5 integer
---@field NonConvert integer
---@field Delete integer
---@field End integer
---@field Help integer
---@field Home integer
---@field Insert integer
---@field PageDown integer
---@field PageUp integer
---@field ArrowDown integer
---@field ArrowLeft integer
---@field ArrowRight integer
---@field ArrowUp integer
---@field NumLock integer
---@field Numpad0 integer
---@field Numpad1 integer
---@field Numpad2 integer
---@field Numpad3 integer
---@field Numpad4 integer
---@field Numpad5 integer
---@field Numpad6 integer
---@field Numpad7 integer
---@field Numpad8 integer
---@field Numpad9 integer
---@field NumpadAdd integer
---@field NumpadBackspace integer
---@field NumpadClear integer
---@field NumpadClearEntry integer
---@field NumpadComma integer
---@field NumpadDecimal integer
---@field NumpadDivide integer
---@field NumpadEnter integer
---@field NumpadEqual integer
---@field NumpadHash integer
---@field NumpadMemoryAdd integer
---@field NumpadMemoryClear integer
---@field NumpadMemoryRecall integer
---@field NumpadMemoryStore integer
---@field NumpadMemorySubtract integer
---@field NumpadMultiply integer
---@field NumpadParenLeft integer
---@field NumpadParenRight integer
---@field NumpadStar integer
---@field NumpadSubtract integer
---@field Escape integer
---@field Fn integer
---@field FnLock integer
---@field PrintScreen integer
---@field ScrollLock integer
---@field Pause integer
---@field BrowserBack integer
---@field BrowserFavorites integer
---@field BrowserForward integer
---@field BrowserHome integer
---@field BrowserRefresh integer
---@field BrowserSearch integer
---@field BrowserStop integer
---@field Eject integer
---@field LaunchApp1 integer
---@field LaunchApp2 integer
---@field LaunchMail integer
---@field MediaPlayPause integer
---@field MediaSelect integer
---@field MediaStop integer
---@field MediaTrackNext integer
---@field MediaTrackPrevious integer
---@field Power integer
---@field Sleep integer
---@field AudioVolumeDown integer
---@field AudioVolumeMute integer
---@field AudioVolumeUp integer
---@field WakeUp integer
---@field Meta integer
---@field Hyper integer
---@field Turbo integer
---@field Abort integer
---@field Resume integer
---@field Suspend integer
---@field Again integer
---@field Copy integer
---@field Cut integer
---@field Find integer
---@field Open integer
---@field Paste integer
---@field Props integer
---@field Select integer
---@field Undo integer
---@field Hiragana integer
---@field Katakana integer
---@field F1 integer
---@field F2 integer
---@field F3 integer
---@field F4 integer
---@field F5 integer
---@field F6 integer
---@field F7 integer
---@field F8 integer
---@field F9 integer
---@field F10 integer
---@field F11 integer
---@field F12 integer
---@field F13 integer
---@field F14 integer
---@field F15 integer
---@field F16 integer
---@field F17 integer
---@field F18 integer
---@field F19 integer
---@field F20 integer
---@field F21 integer
---@field F22 integer
---@field F23 integer
---@field F24 integer
---@field F25 integer
---@field F26 integer
---@field F27 integer
---@field F28 integer
---@field F29 integer
---@field F30 integer
---@field F31 integer
---@field F32 integer
---@field F33 integer
---@field F34 integer
---@field F35 integer
KeyCode = KeyCode

KeyCode = {
    Backquote = 0,
    Section = 1,
    Backslash = 2,
    BracketLeft = 3,
    BracketRight = 4,
    Comma = 5,
    Digit0 = 6,
    Digit1 = 7,
    Digit2 = 8,
    Digit3 = 9,
    Digit4 = 10,
    Digit5 = 11,
    Digit6 = 12,
    Digit7 = 13,
    Digit8 = 14,
    Digit9 = 15,
    Equal = 16,
    IntlBackslash = 17,
    IntlRo = 18,
    IntlYen = 19,
    KeyA = 20,
    KeyB = 21,
    KeyC = 22,
    KeyD = 23,
    KeyE = 24,
    KeyF = 25,
    KeyG = 26,
    KeyH = 27,
    KeyI = 28,
    KeyJ = 29,
    KeyK = 30,
    KeyL = 31,
    KeyM = 32,
    KeyN = 33,
    KeyO = 34,
    KeyP = 35,
    KeyQ = 36,
    KeyR = 37,
    KeyS = 38,
    KeyT = 39,
    KeyU = 40,
    KeyV = 41,
    KeyW = 42,
    KeyX = 43,
    KeyY = 44,
    KeyZ = 45,
    Minus = 46,
    Period = 47,
    Quote = 48,
    Semicolon = 49,
    Slash = 50,
    AltLeft = 51,
    AltRight = 52,
    Backspace = 53,
    CapsLock = 54,
    ContextMenu = 55,
    ControlLeft = 56,
    ControlRight = 57,
    Enter = 58,
    SuperLeft = 59,
    SuperRight = 60,
    ShiftLeft = 61,
    ShiftRight = 62,
    Space = 63,
    Tab = 64,
    Convert = 65,
    KanaMode = 66,
    Lang1 = 67,
    Lang2 = 68,
    Lang3 = 69,
    Lang4 = 70,
    Lang5 = 71,
    NonConvert = 72,
    Delete = 73,
    End = 74,
    Help = 75,
    Home = 76,
    Insert = 77,
    PageDown = 78,
    PageUp = 79,
    ArrowDown = 80,
    ArrowLeft = 81,
    ArrowRight = 82,
    ArrowUp = 83,
    NumLock = 84,
    Numpad0 = 85,
    Numpad1 = 86,
    Numpad2 = 87,
    Numpad3 = 88,
    Numpad4 = 89,
    Numpad5 = 90,
    Numpad6 = 91,
    Numpad7 = 92,
    Numpad8 = 93,
    Numpad9 = 94,
    NumpadAdd = 95,
    NumpadBackspace = 96,
    NumpadClear = 97,
    NumpadClearEntry = 98,
    NumpadComma = 99,
    NumpadDecimal = 100,
    NumpadDivide = 101,
    NumpadEnter = 102,
    NumpadEqual = 103,
    NumpadHash = 104,
    NumpadMemoryAdd = 105,
    NumpadMemoryClear = 106,
    NumpadMemoryRecall = 107,
    NumpadMemoryStore = 108,
    NumpadMemorySubtract = 109,
    NumpadMultiply = 110,
    NumpadParenLeft = 111,
    NumpadParenRight = 112,
    NumpadStar = 113,
    NumpadSubtract = 114,
    Escape = 115,
    Fn = 116,
    FnLock = 117,
    PrintScreen = 118,
    ScrollLock = 119,
    Pause = 120,
    BrowserBack = 121,
    BrowserFavorites = 122,
    BrowserForward = 123,
    BrowserHome = 124,
    BrowserRefresh = 125,
    BrowserSearch = 126,
    BrowserStop = 127,
    Eject = 128,
    LaunchApp1 = 129,
    LaunchApp2 = 130,
    LaunchMail = 131,
    MediaPlayPause = 132,
    MediaSelect = 133,
    MediaStop = 134,
    MediaTrackNext = 135,
    MediaTrackPrevious = 136,
    Power = 137,
    Sleep = 138,
    AudioVolumeDown = 139,
    AudioVolumeMute = 140,
    AudioVolumeUp = 141,
    WakeUp = 142,
    Meta = 143,
    Hyper = 144,
    Turbo = 145,
    Abort = 146,
    Resume = 147,
    Suspend = 148,
    Again = 149,
    Copy = 150,
    Cut = 151,
    Find = 152,
    Open = 153,
    Paste = 154,
    Props = 155,
    Select = 156,
    Undo = 157,
    Hiragana = 158,
    Katakana = 159,
    F1 = 160,
    F2 = 161,
    F3 = 162,
    F4 = 163,
    F5 = 164,
    F6 = 165,
    F7 = 166,
    F8 = 167,
    F9 = 168,
    F10 = 169,
    F11 = 170,
    F12 = 171,
    F13 = 172,
    F14 = 173,
    F15 = 174,
    F16 = 175,
    F17 = 176,
    F18 = 177,
    F19 = 178,
    F20 = 179,
    F21 = 180,
    F22 = 181,
    F23 = 182,
    F24 = 183,
    F25 = 184,
    F26 = 185,
    F27 = 186,
    F28 = 187,
    F29 = 188,
    F30 = 189,
    F31 = 190,
    F32 = 191,
    F33 = 192,
    F34 = 193,
    F35 = 194,
}