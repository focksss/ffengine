use std::path::Path;
use ash::vk;
use mlua;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use mlua::AsChunk;
use crate::app::EngineRef;
use crate::gui::gui::GUI;
use crate::math::Vector;
use crate::physics::player::MovementMode;
use crate::scripting::engine_api::client_api::controller_api::LuaKeyCode;
use crate::scripting::engine_api::engine_api;
use crate::scripting::engine_api::gui_api::gui_api::{GUIRef};

thread_local! {
    static LUA: RefCell<Option<Lua>> = RefCell::new(None);
}

pub struct Script {
    name: String,
    environment: mlua::RegistryKey,
    start_fn: Option<mlua::RegistryKey>,
    update_fn: Option<mlua::RegistryKey>,
    has_started: bool,
}

pub struct Lua {
    lua: mlua::Lua,
    scripts: Vec<Script>,
    free_script_indices: Vec<usize>,

    cached_calls: Vec<(usize, String, usize)>, // cached script calls, stores script index, method name, and call index. Maybe add a parameter cache using a sort of heap?
}

pub trait RegisterToLua {
    fn register_to_lua(lua: &mlua::Lua) -> mlua::Result<()>;
}

impl Lua {
    pub fn initialize(engine: EngineRef) -> Result<(), mlua::Error> {
        LUA.with(|script_engine| {
            if script_engine.borrow().is_some() {
                return Err(mlua::Error::RuntimeError("Lua already initialized".into()));
            }
            *script_engine.borrow_mut() = Some(Self {
                lua: mlua::Lua::new(),
                scripts: Vec::new(),
                free_script_indices: Vec::new(),
                cached_calls: Vec::new(),
            });

            let script_engine_ref = script_engine.borrow();
            let lua = &script_engine_ref.as_ref().unwrap().lua;


            LuaKeyCode::register_to_lua(lua)?;
            MovementMode::register_to_lua(&lua)?;
            Vector::register_to_lua(&lua)?;


            let engine_ud = lua.create_userdata(engine)?;
            lua.globals().set("Engine", engine_ud)?;
            lua.globals().set("dt", 0.0)?;
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
        let on_awake_fn = environment.get::<_, Option<mlua::Function>>("Awake")?
            .map(|f| self.lua.create_registry_value(f))
            .transpose()?;

        if let Some(awake_fn) = &on_awake_fn {
            self.call_method_by_key(awake_fn).expect("failed to call Awake method");
        }

        let script = Script {
            name: path.name().unwrap(),
            environment: self.lua.create_registry_value(environment).unwrap(),
            start_fn,
            update_fn,
            has_started: false,
        };

        let assigned_index = if let Some(index) = self.free_script_indices.pop() {
            self.scripts[index] = script;
            index
        } else {
            self.scripts.push(script);
            self.scripts.len() - 1
        };

        Ok(assigned_index)
    }
    fn call_method_by_key(
        &self,
        method_key: &mlua::RegistryKey,
    ) -> Result<(), mlua::Error> {
        let func: mlua::Function = self.lua.registry_value(method_key)?;
        func.call::<_, ()>(())?;
        Ok(())
    }

    /// Returns assigned indices of the scripts in the directory
    pub fn load_scripts(scripts_dir: Vec<&Path>) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
        Self::with_mut(|lua| lua.load_scripts_impl(scripts_dir))
    }
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

    pub fn call_script(
        script_index: usize,
        method_name: &str,
    ) -> Result<(), mlua::Error> {
        Self::with_mut(|lua| {
            lua.call_method_impl(script_index, method_name)
        })
    }

    fn call_method_impl(
        &self,
        script_index: usize,
        method_name: &str,
    ) -> Result<(), mlua::Error> {
        let script = &self.scripts[script_index];

        let env: mlua::Table = self.lua.registry_value(&script.environment)?;
        let method: Option<mlua::Function> = env.get(method_name)?;

        match method {
            Some(func) => {
                let key = self.lua.create_registry_value(func)?;
                let method: mlua::Function = self.lua.registry_value(&key)?;

                method.call::<_, ()>(())
            }
            None => {
                Err(mlua::Error::RuntimeError(
                    format!("Method '{}' not found in script '{}'", method_name, script.name)
                ))
            }
        }
    }

    /// call_index is used as the "active_object" index during method calling
    pub fn cache_call(script_index: usize, method_name: &str, call_index: Option<usize>) {
        Self::with_mut(|lua| lua.cache_call_impl(script_index, method_name, call_index));
    }
    fn cache_call_impl(
        &mut self,
        script_index: usize,
        method_name: &str,
        call_index: Option<usize>,
    ) {
        self.cached_calls.push((script_index, method_name.to_string(), call_index.unwrap_or(0)));
    }

    pub fn run_cache(
        engine: &EngineRef,
    ) {
        Self::with_mut(|lua| {
            lua.run_cache_impl(engine);
        })
    }
    fn run_cache_impl(
        &mut self,
        engine: &EngineRef,
    ) {
        for call in self.cached_calls.clone() {
            engine.renderer.borrow_mut().gui.borrow_mut().active_node = call.2;
            self.call_method_impl(
                call.0,
                call.1.as_str(),
            ).expect("failed to call cached method");
        }
        self.cached_calls.clear();
    }

    fn run_update_methods_impl(&mut self) -> Result<(), mlua::Error> {
        for i in 0..self.scripts.len() {
            if self.scripts[i].update_fn.is_some() {
                if !self.scripts[i].has_started {
                    self.scripts[i].has_started = true;
                    if let Some(start_fn) = &self.scripts[i].start_fn {
                        self.call_method_by_key(start_fn)?
                    }
                }
                self.call_method_by_key(self.scripts[i].update_fn.as_ref().unwrap())?
            }
        }
        Ok(())
    }
    pub fn run_update_methods() -> Result<(), mlua::Error> {
        Self::with_mut(|lua| {lua.run_update_methods_impl()})
    }

    pub fn with_lua<F, R>(f: F) -> R
    where F: FnOnce(&mlua::Lua) -> R, {
        Self::with(|lua_engine| f(&lua_engine.lua))
    }
}