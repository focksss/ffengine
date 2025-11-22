---@meta

---@class MovementMode
MovementMode = {
    GHOST = 0,
    PHYSICS = 1,
    EDITOR = 2,
}

---@class GUIQuad
---@field set_color fun(self:GUIQuad, r:number, g:number, b:number, a:number):nil

---@class GUIText
---@field text_message string
---@field update_text fun(self:GUIText, text:string):nil

---@class GUINode
---@field quad GUIQuad
---@field text GUIText

---@class GUI
---@field ActiveNode GUINode

---@class Renderer
---@field gui GUI

---@class Flags
---@field reload_gui_queued boolean
---@field reload_shaders_queued boolean
---@field pause_rendering boolean
---@field screenshot_queued boolean
---@field draw_hitboxes boolean
---@field do_physics boolean
---@field reload_all_scripts_queued boolean

---@class MouseButton
MouseButton = {
    Left = 0,
    Right = 1,
    Middle = 2,
    Back = 3,
    Forward = 4,
}

---@class Controller
---@field flags Flags
---@field cursor_position Vector
---@field scroll_delta Vector
---@field mouse_delta Vector
---@field cursor_locked boolean
---@field window_size Vector
---@field ButtonPressed MouseButton
---@field ButtonReleased MouseButton
---@field new_key_pressed fun(self:Controller, key: number):boolean
---@field key_pressed fun(self:Controller, key: number):boolean
---@field mouse_button_pressed fun(self:Controller, button: number):boolean

---@class Camera
---@field position Vector
---@field rotation Vector
---@field fov_y number
---@field aspect_ratio number
---@field near number
---@field far number

---@class RigidBody
---@field position Vector
---@field velocity Vector

---@class Player
---@field movement_mode number
---@field grounded boolean
---@field rigid_body RigidBody
---@field camera Camera

---@class PhysicsEngine
---@field gravity Vector
---@field get_player fun(self:PhysicsEngine, index:number):Player
---@field get_rigid_body fun(self:PhysicsEngine, index:number):RigidBody

---@class EngineClass
---@field renderer Renderer
---@field controller Controller
---@field physics_engine PhysicsEngine

---@type EngineClass
Engine = nil

---@type number
dt = 0

---@class Vector
---@operator add(Vector): Vector
---@operator sub(Vector): Vector
---@operator mul(Vector|number): Vector
---@operator div(Vector): Vector
---@operator unm: Vector
---@field x number
---@field y number
---@field z number
---@field w number

---@param x number
---@param y number
---@param z number
---@param w number
---@return Vector
function Vector.new_vec4(x, y, z, w) end

---@param x number
---@param y number
---@param z number
---@return Vector
function Vector.new_vec3(x, y, z) end

---@param x number
---@param y number
---@return Vector
function Vector.new_vec2(x, y) end

---@param x number
---@return Vector
function Vector.new_vec(x) end

---@return Vector
function Vector.new() end

---@return Vector
function Vector.new_empty_quat() end

---@return Vector
function Vector:normalize_3d() end

---@param rot Vector
---@return Vector
function Vector:rotate_by_euler(rot) end

Vector = Vector

---@class KeyCode
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