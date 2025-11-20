use std::path::Path;
use ash::vk;
use mlua;
use std::cell::RefCell;
use std::collections::HashMap;
use mlua::AsChunk;
use crate::gui::gui::GUI;
use crate::scripting::engine_context::controller_access::ScriptController;
use crate::scripting::engine_context::gui_access::ScriptGUI;

thread_local! {
    static LUA: RefCell<Option<Lua>> = RefCell::new(None);
}

pub struct Script {
    name: String,
    environment: mlua::RegistryKey,
    start_fn: Option<mlua::RegistryKey>,
    update_fn: Option<mlua::RegistryKey>,
    on_awake_fn: Option<mlua::RegistryKey>,
    has_started: bool,
}

pub struct Lua {
    lua: mlua::Lua,
    scripts: Vec<Script>,
    free_script_indices: Vec<usize>,
}

impl Lua {
    pub fn initialize() -> Result<(), mlua::Error> {
        LUA.with(|engine| {
            if engine.borrow().is_some() {
                return Err(mlua::Error::RuntimeError("LUA already initialized".into()));
            }
            *engine.borrow_mut() = Some(Self {
                lua: mlua::Lua::new(),
                scripts: Vec::new(),
                free_script_indices: Vec::new(),
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

    /// Returns the script and assigned index (could be different from scripts.len())
    fn load_script_impl(&mut self, path: &Path) -> Result<(usize), Box<dyn std::error::Error>> {
        let script_content = std::fs::read_to_string(path)?;
        // create local-environment
        let environment = self.lua.create_table()?;
        // create a metadata-table describing local access to global
        let metatable = self.lua.create_table()?;
        metatable.set("__index", self.lua.globals())?;
        // apply that metadata-table to the local environment
        environment.set_metatable(Some(metatable));

        self.lua.load(script_content)
            .set_environment(environment.clone())
            .exec()?;

        // extract lifecycle functions
        let start_fn = environment.get::<_, Option<mlua::Function>>("Start")?
            .map(|f| self.lua.create_registry_value(f))
            .transpose()?;
        let update_fn = environment.get::<_, Option<mlua::Function>>("Update")?
            .map(|f| self.lua.create_registry_value(f))
            .transpose()?;
        let on_awake_fn = environment.get::<_, Option<mlua::Function>>("Update")?
            .map(|f| self.lua.create_registry_value(f))
            .transpose()?;

        let script = Script {
            name: path.name().unwrap(),
            environment: self.lua.create_registry_value(environment).unwrap(),
            start_fn,
            update_fn,
            on_awake_fn,
            has_started: false,
        };

        let assigned_index = if let Some(index) = self.free_script_indices.pop() {
            self.scripts[index] = script;
            index
        } else {
            self.scripts.push(script);
            self.scripts.len() - 1
        };

        // TODO: Call on_awake_fn via a method_call function, bypassing an automatic "set already_started to true"

        Ok(assigned_index)
    }

    /// Returns assigned indices of the scripts in the directory
    fn load_scripts_impl(&mut self, scripts_dir: Vec<&Path>) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
        for script in self.scripts.drain(..) {
            self.lua.remove_registry_value(script.environment)?;
            if let Some(start_fn) = script.start_fn {
                self.lua.remove_registry_value(start_fn)?;
            }
            if let Some(update_fn) = script.update_fn {
                self.lua.remove_registry_value(update_fn)?;
            }
        }

        let mut indices = Vec::new();

        for path in scripts_dir {
            if path.extension().and_then(|s| s.to_str()) == Some("lua") {
                indices.push(self.load_script_impl(path)?);
            }
        }
        Ok(indices)
    }

    fn call_method_impl<'lua>(
        &'lua self,
        script_index: usize,
        method_name: &str,
        // context: Script, TODO: send the lua exposed engine
        // TODO: remove the below contexts
        gui: &mut GUI,
        command_buffer: vk::CommandBuffer,
    ) -> Result<(), mlua::Error> {
        let script = &self.scripts[script_index];

        let env: mlua::Table = self.lua.registry_value(&script.environment)?;
        let method: Option<mlua::Function> = env.get(method_name)?;

        match method {
            Some(func) => {
                let key = self.lua.create_registry_value(func)?;
                let env: mlua::Table = self.lua.registry_value(&script.environment)?;
                let method: mlua::Function = self.lua.registry_value(&key)?;

                let controller = ScriptController(gui.controller.clone());
                let script_gui = ScriptGUI { gui, command_buffer };

                self.lua.scope(|scope| {
                    // TODO: refer to above TODOs
                    // env.set("Engine", scope.create_nonstatic_userdata(context)?)?;

                    let gui_ud = scope.create_nonstatic_userdata(script_gui)?;
                    let controller_ud = scope.create_userdata(controller)?;

                    self.lua.globals().set("GUI", gui_ud)?;
                    self.lua.globals().set("controller", controller_ud)?;

                    method.call::<_, ()>(())
                })
            }
            None => {
                Err(mlua::Error::RuntimeError(
                    format!("Method '{}' not found in script '{}'", method_name, script.name)
                ))
            }
        }
    }

    /// Returns assigned indices of the scripts in the directory
    pub fn load_scripts(scripts_dir: Vec<&Path>) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
        Self::with_mut(|engine| engine.load_scripts_impl(scripts_dir))
    }

    pub fn call_script(
        script_index: usize,
        method_name: &str,
        gui: &mut GUI,
        command_buffer: vk::CommandBuffer,
    ) -> Result<(), mlua::Error> {
        Self::with_mut(|engine| {
            engine.call_method_impl(script_index, method_name, gui, command_buffer)
        })
    }

    pub fn with_lua<F, R>(f: F) -> R
    where F: FnOnce(&mlua::Lua) -> R, {
        Self::with(|engine| f(&engine.lua))
    }
}