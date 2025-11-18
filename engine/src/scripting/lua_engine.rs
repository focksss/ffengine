use std::path::{Path, PathBuf};
use ash::vk;
use mlua::Lua;
use crate::gui::gui::{ScriptGUI, GUI};
use crate::physics::controller::ScriptController;

pub struct LuaScriptEngine {
    lua: Lua,
    pub(crate) scripts: Vec<String>, // action_name, script source
}

impl LuaScriptEngine {
    pub fn new() -> Result<Self, mlua::Error> {
        let lua = Lua::new();

        Ok(Self {
            lua,
            scripts: Vec::new(),
        })
    }

    pub fn load_script(&mut self, script_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let script_content = std::fs::read_to_string(script_path)?;
        self.scripts.push(script_content);
        Ok(())
    }

    pub fn load_scripts(&mut self, scripts_dir: Vec<&Path>) -> Result<(), Box<dyn std::error::Error>> {
        self.scripts.clear();

        if scripts_dir.is_empty() {
            return Ok(());
        }

        for path in scripts_dir {
            if path.extension().and_then(|s| s.to_str()) == Some("lua") {
                let action_name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                self.load_script(&path)?;
            }
        }
        Ok(())
    }

    pub fn call_script(
        &self,
        action_index: usize,
        gui: &mut GUI,
        node_index: usize,
        command_buffer: vk::CommandBuffer,
    ) -> Result<(), mlua::Error> {
        if let script_source = &self.scripts[action_index] {
            let controller = ScriptController(gui.controller.clone());
            let script_gui = ScriptGUI {
                gui,
                command_buffer,
            };


            let result = self.lua.scope(|scope| {
                let gui_ud = scope.create_nonstatic_userdata(script_gui)?;
                let controller_ud = scope.create_userdata(controller)?;

                self.lua.globals().set("gui", gui_ud)?;
                self.lua.globals().set("controller", controller_ud)?;
                self.lua.globals().set("node_index", node_index)?;

                self.lua.load(script_source).exec()
            });

            if let Err(e) = result {
                eprintln!("Lua script error in '{}': {}", action_index, e);
                return Err(e);
            }
        } else {
            eprintln!("Script not found: {}", action_index);
        }
        Ok(())
    }
}