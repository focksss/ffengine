use std::path::Path;
use ash::vk;
use mlua;
use crate::gui::gui::{ScriptGUI, GUI};
use crate::client::controller::ScriptController;
use std::cell::RefCell;

thread_local! {
    static LUA: RefCell<Option<Lua>> = RefCell::new(None);
}

pub struct Lua {
    lua: mlua::Lua,
    script_keys: Vec<mlua::RegistryKey>,
}

impl Lua {
    pub fn initialize() -> Result<(), mlua::Error> {
        LUA.with(|engine| {
            if engine.borrow().is_some() {
                return Err(mlua::Error::RuntimeError("LUA already initialized".into()));
            }
            *engine.borrow_mut() = Some(Self {
                lua: mlua::Lua::new(),
                script_keys: Vec::new(),
            });
            Ok(())
        })
    }

    pub fn with<F, R>(f: F) -> R where F: FnOnce(&Lua) -> R, {
        LUA.with(|engine| {
            let borrowed = engine.borrow();
            f(borrowed.as_ref().expect("LUA_ENGINE not initialized. Call LuaScriptEngine::initialize() first."))
        })
    }

    pub fn with_mut<F, R>(f: F) -> R where F: FnOnce(&mut Lua) -> R, {
        LUA.with(|engine| {
            let mut borrowed = engine.borrow_mut();
            f(borrowed.as_mut().expect("LUA_ENGINE not initialized. Call LuaScriptEngine::initialize() first."))
        })
    }

    fn load_scripts_impl(&mut self, scripts_dir: Vec<&Path>) -> Result<(), Box<dyn std::error::Error>> {
        for key in self.script_keys.drain(..) {
            self.lua.remove_registry_value(key)?;
        }
        for path in scripts_dir {
            if path.extension().and_then(|s| s.to_str()) == Some("lua") {
                let script_content = std::fs::read_to_string(path)?;
                let func: mlua::Function = self.lua.load(&script_content).into_function()?;
                let key = self.lua.create_registry_value(func)?;
                self.script_keys.push(key);
            }
        }
        Ok(())
    }

    fn call_script_impl(
        &self,
        action_index: usize,
        gui: &mut GUI,
        node_index: usize,
        command_buffer: vk::CommandBuffer,
    ) -> Result<(), mlua::Error> {
        let key = &self.script_keys[action_index];
        let func: mlua::Function = self.lua.registry_value(key)?;

        let controller = ScriptController(gui.controller.clone());
        let script_gui = ScriptGUI { gui, command_buffer };

        self.lua.scope(|scope| {
            let gui_ud = scope.create_nonstatic_userdata(script_gui)?;
            let controller_ud = scope.create_userdata(controller)?;

            self.lua.globals().set("gui", gui_ud)?;
            self.lua.globals().set("controller", controller_ud)?;
            self.lua.globals().set("node_index", node_index)?;

            func.call::<_, ()>(())
        })
    }

    pub fn load_scripts(scripts_dir: Vec<&Path>) -> Result<(), Box<dyn std::error::Error>> {
        Self::with_mut(|engine| engine.load_scripts_impl(scripts_dir))
    }

    pub fn call_script(
        action_index: usize,
        gui: &mut GUI,
        node_index: usize,
        command_buffer: vk::CommandBuffer,
    ) -> Result<(), mlua::Error> {
        Self::with(|engine| engine.call_script_impl(action_index, gui, node_index, command_buffer))
    }
}