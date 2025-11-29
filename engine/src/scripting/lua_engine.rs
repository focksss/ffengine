use std::path::{Path, PathBuf};
use ash::vk;
use mlua;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use mlua::AsChunk;
use crate::engine::EngineRef;
use crate::gui::gui::GUI;
use crate::math::Vector;
use crate::scene::physics::player::MovementMode;
use crate::scripting::engine_api::client_api::client_api::{LuaCursorIcon, LuaKeyCode, LuaMouseButton, LuaResizeDirection};
use crate::scripting::engine_api::engine_api;
use crate::scripting::engine_api::gui_api::gui_api::{LuaAnchorPoint};

thread_local! {
    static LUA: RefCell<Option<Lua>> = RefCell::new(None);
}

pub struct Script {
    path: PathBuf,
    environment: mlua::RegistryKey,
    start_fn: Option<mlua::RegistryKey>,
    update_fn: Option<mlua::RegistryKey>,
    scroll_fn: Option<mlua::RegistryKey>,
    mouse_moved_fn: Option<mlua::RegistryKey>,
    mouse_button_pressed_fn: Option<mlua::RegistryKey>,
    mouse_button_released_fn: Option<mlua::RegistryKey>,
    has_started: bool,
}

pub struct Lua {
    lua: mlua::Lua,
    scripts: Vec<Script>,
    free_script_indices: Vec<usize>,

    cached_calls: Vec<(usize, String, usize, usize)>, // stores script index, method name, call index, and gui index.
    reload_scripts_requested: bool,
    load_scripts_requested: bool,
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
                reload_scripts_requested: false,
                load_scripts_requested: false,
            });

            let script_engine_ref = script_engine.borrow();
            let lua = &script_engine_ref.as_ref().unwrap().lua;

            lua.set_app_data(engine.clone());

            LuaAnchorPoint::register_to_lua(lua)?;
            LuaCursorIcon::register_to_lua(lua)?;
            LuaResizeDirection::register_to_lua(lua)?;
            LuaMouseButton::register_to_lua(lua)?;
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

    pub fn force_gc() {
        Self::with(|lua| {lua.force_gc_impl()})
    }
    fn force_gc_impl(&self) {
        self.lua.gc_collect().unwrap()
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
        let scroll_fn = environment.get::<_, Option<mlua::Function>>("MouseScrolled")?
            .map(|f| self.lua.create_registry_value(f))
            .transpose()?;
        let mouse_moved_fn = environment.get::<_, Option<mlua::Function>>("MouseMoved")?
            .map(|f| self.lua.create_registry_value(f))
            .transpose()?;
        let mouse_button_pressed_fn = environment.get::<_, Option<mlua::Function>>("MouseButtonPressed")?
            .map(|f| self.lua.create_registry_value(f))
            .transpose()?;
        let mouse_button_released_fn = environment.get::<_, Option<mlua::Function>>("MouseButtonReleased")?
            .map(|f| self.lua.create_registry_value(f))
            .transpose()?;

        let script = Script {
            path: PathBuf::from(path),
            environment: self.lua.create_registry_value(environment).unwrap(),
            start_fn,
            update_fn,
            scroll_fn,
            mouse_moved_fn,
            mouse_button_pressed_fn,
            mouse_button_released_fn,
            has_started: false,
        };

        let assigned_index = if let Some(index) = self.free_script_indices.pop() {
            self.scripts[index] = script;
            index
        } else {
            self.scripts.push(script);
            self.scripts.len() - 1
        };

        if on_awake_fn.is_some() {
            self.cache_call_impl(assigned_index, "Awake", None, None);
        }

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
    fn load_scripts_impl(
        &mut self,
        scripts_dir: Vec<&Path>,
    ) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
        let scripts_dir_bufs: Vec<PathBuf> = scripts_dir.iter().map(|p| p.to_path_buf()).collect();

        let to_remove: Vec<usize> = self.scripts
            .iter()
            .enumerate()
            .filter(|(_, s)| scripts_dir_bufs.contains(&s.path))
            .map(|(i, _)| i)
            .collect();

        for &i in to_remove.iter().rev() {
            let script = self.scripts.remove(i);
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

    pub fn reload_scripts() {
        Self::with_mut(|lua| lua.reload_scripts_impl()).expect("failed to reload scripts");
    }
    fn reload_scripts_impl(
        &mut self,
    ) -> Result<(), mlua::Error> {
        let mut scripts_dir = Vec::new();
        for script in self.scripts.drain(..) {
            scripts_dir.push(script.path);
            self.lua.remove_registry_value(script.environment)?;
            if let Some(start_fn) = script.start_fn {
                self.lua.remove_registry_value(start_fn)?;
            }
            if let Some(update_fn) = script.update_fn {
                self.lua.remove_registry_value(update_fn)?;
            }
        }

        for path in scripts_dir {
            if path.extension().and_then(|s| s.to_str()) == Some("lua") {
                self.load_script_impl(Path::new(&path)).expect("failed to load script");
            }
        }
        Ok(())
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
        &mut self,
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
                    format!("Method '{}' not found in script '{}'", method_name, script.path.display()).into(),
                ))
            }
        }
    }

    /// call_index is used as the "active_object" index during method calling
    pub fn cache_call(script_index: usize, method_name: &str, call_index: Option<usize>, gui_index: Option<usize>) {
        Self::with_mut(|lua| lua.cache_call_impl(script_index, method_name, call_index, gui_index));
    }
    fn cache_call_impl(
        &mut self,
        script_index: usize,
        method_name: &str,
        call_index: Option<usize>,
        gui_index: Option<usize>,
    ) {
        self.cached_calls.push((script_index, method_name.to_string(), call_index.unwrap_or(0), gui_index.unwrap_or(0)));
    }

    pub fn clear_cache() {
        Self::with_mut(|lua| lua.cached_calls.clear());
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
            engine.renderer.borrow_mut().guis[call.3].borrow_mut().active_node = call.2;
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

    fn run_scroll_methods_impl(&mut self) -> Result<(), mlua::Error> {
        for i in 0..self.scripts.len() {
            if self.scripts[i].scroll_fn.is_some() {
                self.call_method_by_key(self.scripts[i].scroll_fn.as_ref().unwrap())?
            }
        }
        Ok(())
    }
    pub fn run_scroll_methods() -> Result<(), mlua::Error> {
        Self::with_mut(|lua| {lua.run_scroll_methods_impl()})
    }

    fn run_mouse_moved_methods_impl(&mut self) -> Result<(), mlua::Error> {
        for i in 0..self.scripts.len() {
            if self.scripts[i].mouse_moved_fn.is_some() {
                self.call_method_by_key(self.scripts[i].mouse_moved_fn.as_ref().unwrap())?
            }
        }
        Ok(())
    }
    pub fn run_mouse_moved_methods() -> Result<(), mlua::Error> {
        Self::with_mut(|lua| {lua.run_mouse_moved_methods_impl()})
    }

    fn run_mouse_button_pressed_methods_impl(&mut self) -> Result<(), mlua::Error> {
        for i in 0..self.scripts.len() {
            if self.scripts[i].mouse_button_pressed_fn.is_some() {
                self.call_method_by_key(self.scripts[i].mouse_button_pressed_fn.as_ref().unwrap())?
            }
        }
        Ok(())
    }
    pub fn run_mouse_button_pressed_methods() -> Result<(), mlua::Error> {
        Self::with_mut(|lua| {lua.run_mouse_button_pressed_methods_impl()})
    }

    fn run_mouse_button_released_methods_impl(&mut self) -> Result<(), mlua::Error> {
        for i in 0..self.scripts.len() {
            if self.scripts[i].mouse_button_released_fn.is_some() {
                self.call_method_by_key(self.scripts[i].mouse_button_released_fn.as_ref().unwrap())?
            }
        }
        Ok(())
    }
    pub fn run_mouse_button_released_methods() -> Result<(), mlua::Error> {
        Self::with_mut(|lua| {lua.run_mouse_button_released_methods_impl()})
    }

    pub fn with_lua<F, R>(f: F) -> R
    where F: FnOnce(&mlua::Lua) -> R, {
        Self::with(|lua_engine| f(&lua_engine.lua))
    }
}