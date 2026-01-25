use std::any::TypeId;
use std::path::{Path, PathBuf};
use mlua;
use std::cell::RefCell;
use std::collections::HashMap;
use mlua::{Function, IntoLua, Value};
use crate::engine::EngineRef;
use crate::math::Vector;
use crate::scripting::engine_api::client_api::client_api::{LuaCursorIcon, LuaKeyCode, LuaMouseButton, LuaResizeDirection};
use crate::scripting::engine_api::gui_api::gui_api::{GUIImagePointer, GUINodePointer, GUIQuadPointer, GUITextPointer, GUITexturePointer, LuaAnchorPoint};
use crate::scripting::engine_api::scene_api::scene_api::{CameraPointer, EntityPointer, RenderComponentPointer, RigidBodyPointer, TransformPointer};

thread_local! {
    static LUA: RefCell<Option<Lua>> = RefCell::new(None);
}

pub struct ScriptAsset {
    path: PathBuf,
    fields: HashMap<String, FieldType>,

    instances: Vec<ScriptInstance>,
}
enum FieldType {
    Number,
    Integer,
    String,
    Boolean,
    Table,
    UserData(TypeId),
    Unknown,
}
#[derive(Clone)]
pub enum Field {
    Number(f32),
    Integer(i32),
    String(String),
    Boolean(bool),
    Table(Vec<Box<Field>>),
    UiNode(GUINodePointer),
    UiQuad(GUIQuadPointer),
    UiText(GUITextPointer),
    UiTexture(GUITexturePointer),
    UiImage(GUIImagePointer),
    Entity(EntityPointer),
    Transform(TransformPointer),
    RenderComponent(RenderComponentPointer),
    RigidBody(RigidBodyPointer),
    Camera(CameraPointer),
}
fn field_to_value(lua: &mlua::Lua, field: Field) -> Result<mlua::Value, mlua::Error> {
    Ok(match field {
        Field::Number(v) => mlua::Value::Number(v as f64),
        Field::Integer(v) => mlua::Value::Integer(v as i64),
        Field::String(v) => mlua::Value::String(lua.create_string(&v)?),
        Field::Boolean(v) => mlua::Value::Boolean(v),
        Field::Table(fields) => {
            let table = lua.create_table()?;
            for (i, field) in fields.into_iter().enumerate() {
                table.set(i + 1, field_to_value(lua, *field)?)?;
            }
            mlua::Value::Table(table)
        }
        Field::UiNode(v) => mlua::Value::UserData(lua.create_userdata(v)?),
        Field::UiQuad(v) => mlua::Value::UserData(lua.create_userdata(v)?),
        Field::UiText(v) => mlua::Value::UserData(lua.create_userdata(v)?),
        Field::UiTexture(v) => mlua::Value::UserData(lua.create_userdata(v)?),
        Field::UiImage(v) => mlua::Value::UserData(lua.create_userdata(v)?),
        Field::Entity(v) => mlua::Value::UserData(lua.create_userdata(v)?),
        Field::Transform(v) => mlua::Value::UserData(lua.create_userdata(v)?),
        Field::RenderComponent(v) => mlua::Value::UserData(lua.create_userdata(v)?),
        Field::RigidBody(v) => mlua::Value::UserData(lua.create_userdata(v)?),
        Field::Camera(v) => mlua::Value::UserData(lua.create_userdata(v)?),
    })
}
impl IntoLua<'_> for Field {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<Value> {
        field_to_value(lua, self)
    }
}

struct ScriptInstance {
    owner: EntityPointer,
    environment: mlua::RegistryKey,
    has_started: bool,
}

pub struct Lua {
    lua: mlua::Lua,
    scripts: Vec<ScriptAsset>,
    free_script_indices: Vec<usize>,

    reload_scripts_requested: bool,
    load_scripts_requested: bool,
}

pub trait RegisterToLua {
    fn register_to_lua(lua: &mlua::Lua) -> mlua::Result<()>;
}


/*
let env: mlua::Table = self.lua.registry_value(&self.scripts[i].environment)?;
for pair in env.pairs::<mlua::Value, mlua::Value>() {
let (key, value) = pair?;

// Only string keys are meaningful for variable names
if let mlua::Value::String(key_str) = key {
let name = key_str.to_str()?;

// Optional: skip internal / metadata fields
if name.starts_with("__") || !name.contains("editor_distance") {
continue;
}

println!("{} = {}", name, format_lua_value(&value));
}
}

*/

fn format_lua_value(value: &mlua::Value) -> String {
    match value {
        mlua::Value::Nil => "nil".to_string(),
        mlua::Value::Boolean(b) => b.to_string(),
        mlua::Value::Integer(i) => i.to_string(),
        mlua::Value::Number(n) => n.to_string(),
        mlua::Value::String(s) => format!("{:?}", s.to_str().unwrap_or("<invalid utf8>")),
        mlua::Value::Table(_) => "<table>".to_string(),
        mlua::Value::Function(_) => "<function>".to_string(),
        mlua::Value::UserData(_) => "<userdata>".to_string(),
        mlua::Value::Thread(_) => "<thread>".to_string(),
        mlua::Value::LightUserData(_) => "<lightuserdata>".to_string(),
        mlua::Value::Error(e) => format!("<error: {}>", e),
    }
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
    fn load_script_impl(&mut self, path: &Path) -> Result<usize, Box<dyn std::error::Error>> {
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

        let mut fields = HashMap::new();

        let field_type = |value: &mlua::Value| match value {
            mlua::Value::Integer(_) => FieldType::Integer,
            mlua::Value::Number(_) => FieldType::Number,
            mlua::Value::Boolean(_) => FieldType::Boolean,
            mlua::Value::String(_) => FieldType::String,
            mlua::Value::Table(_) => FieldType::Table,
            // mlua::Value::UserData(ud) => FieldType::UserData(ud.type_id()),
            _ => FieldType::Unknown,
        };

        for pair in environment.clone().pairs::<mlua::Value, mlua::Value>() {
            let (key, value) = pair?;

            let key_str = match key {
                mlua::Value::String(s) => s.to_str()?.to_string(),
                _ => continue,
            };

            match value {
                mlua::Value::UserData(_)
                | mlua::Value::Integer(_)
                | mlua::Value::Number(_)
                | mlua::Value::Boolean(_)
                | mlua::Value::String(_)
                | mlua::Value::Table(_) => {
                    let field_type = field_type(&value);
                    fields.insert(key_str, field_type);
                },
                _ => {}
            }
        }

        let script = ScriptAsset {
            path: PathBuf::from(path),
            fields,
            instances: Vec::new(),
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

    pub fn add_script_instance(script_index: usize, owner: EntityPointer) {
        Self::with_mut(|lua| { lua.add_script_instance_impl(script_index, owner).expect("script instance add failed"); })
    }
    fn add_script_instance_impl(&mut self, script_index: usize, owner: EntityPointer) -> Result<(), mlua::Error> {
        let script = &mut self.scripts[script_index];

        // create local-environment
        let environment = self.lua.create_table()?;
        // create a metadata-table describing local access to global
        let metatable = self.lua.create_table()?;
        metatable.set("__index", self.lua.globals())?;
        // apply that metadata-table to the local environment
        environment.set_metatable(Some(metatable));

        if let Some(update) = environment.get::<_, Option<Function>>("Update")? {
            update.call::<_, ()>(())?;
        }

        script.instances.push(ScriptInstance {
            owner,
            environment: self.lua.create_registry_value(environment)?,
            has_started: false,
        });
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
            self.scripts.remove(i);
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
        }

        for path in scripts_dir {
            if path.extension().and_then(|s| s.to_str()) == Some("lua") {
                self.load_script_impl(Path::new(&path)).expect("failed to load script");
            }
        }
        Ok(())
    }

    pub fn call_method(
        script_index: usize,
        instance_index: usize,
        method_name: &str,
        args: &Vec<Field>,
    ) -> Result<(), mlua::Error> {
        Self::with_mut(|lua| {
            lua.call_method_impl(script_index, instance_index, method_name, args)
        })
    }
    fn call_method_impl(
        &mut self,
        script_index: usize,
        instance_index: usize,
        method_name: &str,
        args: &Vec<Field>,
    ) -> Result<(), mlua::Error> {
        let script = &self.scripts[script_index];
        let environment: mlua::Table = self.lua.registry_value(&script.instances[instance_index].environment)?;

        let args = args
            .into_iter()
            .map(|f| f.clone().into_lua(&self.lua).expect("arg parse error"))
            .collect::<Vec<Value>>();
        let args = mlua::MultiValue::from_vec(args);

        if let Some(func) = environment.get::<_, Option<Function>>(method_name)? {
            func.call::<_, ()>(args)?;
            Ok(())
        } else {
            panic!("failed to call method {}", method_name);
        }
    }

    /// call_index is used as the "active_object" index during method calling

    fn run_update_methods_impl(&mut self) -> Result<(), mlua::Error> {
        for script_index in 0..self.scripts.len() {
            for instance_index in 0..self.scripts[script_index].instances.len() {
                self.call_method_impl(script_index, instance_index, "Update", &Vec::new())?;
            }
        }
        Ok(())
    }
    pub fn run_update_methods() -> Result<(), mlua::Error> {
        Self::with_mut(|lua| {lua.run_update_methods_impl()})
    }

    fn run_scroll_methods_impl(&mut self) -> Result<(), mlua::Error> {
        for script_index in 0..self.scripts.len() {
            for instance_index in 0..self.scripts[script_index].instances.len() {
                self.call_method_impl(script_index, instance_index, "MouseScrolled", &Vec::new())?;
            }
        }
        Ok(())
    }
    pub fn run_scroll_methods() -> Result<(), mlua::Error> {
        Self::with_mut(|lua| {lua.run_scroll_methods_impl()})
    }

    fn run_mouse_moved_methods_impl(&mut self) -> Result<(), mlua::Error> {
        for script_index in 0..self.scripts.len() {
            for instance_index in 0..self.scripts[script_index].instances.len() {
                self.call_method_impl(script_index, instance_index, "MouseMoved", &Vec::new())?;
            }
        }
        Ok(())
    }
    pub fn run_mouse_moved_methods() -> Result<(), mlua::Error> {
        Self::with_mut(|lua| {lua.run_mouse_moved_methods_impl()})
    }

    fn run_mouse_button_pressed_methods_impl(&mut self) -> Result<(), mlua::Error> {
        for script_index in 0..self.scripts.len() {
            for instance_index in 0..self.scripts[script_index].instances.len() {
                self.call_method_impl(script_index, instance_index, "MouseButtonPressed", &Vec::new())?;
            }
        }
        Ok(())
    }
    pub fn run_mouse_button_pressed_methods() -> Result<(), mlua::Error> {
        Self::with_mut(|lua| {lua.run_mouse_button_pressed_methods_impl()})
    }

    fn run_mouse_button_released_methods_impl(&mut self) -> Result<(), mlua::Error> {
        for script_index in 0..self.scripts.len() {
            for instance_index in 0..self.scripts[script_index].instances.len() {
                self.call_method_impl(script_index, instance_index, "MouseButtonReleased", &Vec::new())?;
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