use std::collections::HashMap;
use std::fs;
use std::path::Path;
use libloading::{Library, Symbol};

pub struct PluginManager {
    plugins: HashMap<String, Library>,
}

impl PluginManager {
    pub fn new() -> Self {
        PluginManager {
            plugins: HashMap::new(),
        }
    }

    pub fn load_plugins(&mut self, plugin_dir: &Path) -> Result<(), String> {
        // Ensure the plugin directory exists
        if !plugin_dir.exists() {
            return Err(format!("Plugin directory '{}' does not exist.", plugin_dir.display()));
        }
        if !plugin_dir.is_dir() {
            return Err(format!("'{}' is not a directory.", plugin_dir.display()));
        }

        for entry in fs::read_dir(plugin_dir).map_err(|e| format!("Failed to read plugin directory: {}", e))? {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();

            // Determine the expected library extension based on the operating system
            let expected_extension = if cfg!(target_os = "windows") {
                "dll"
            } else if cfg!(target_os = "macos") {
                "dylib" // or "bundle" depending on your use case
            } else { // Assuming Linux and other Unix-like systems
                "so"
            };

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some(expected_extension) {
                let plugin_name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| format!("Invalid plugin filename: '{}'. Filename must have a stem.", path.display()))?
                    .to_string();

                // Construct the full plugin name for error messages
                let full_plugin_path = path.display();

                Library::new(&path)
                    .map_err(|e| format!("Failed to load plugin '{}': {}", full_plugin_path, e))
                    .and_then(|library| {
                        if self.plugins.contains_key(&plugin_name) {
                            Err(format!("Plugin with name '{}' already loaded. Skipping '{}'.", plugin_name, full_plugin_path))
                        } else {
                            self.plugins.insert(plugin_name.clone(), library);
                            println!("Plugin '{}' loaded successfully from '{}'.", plugin_name, full_plugin_path); // Optional success message
                            Ok(())
                        }
                    })?;
            }
        }
        Ok(())
    }

    pub fn get_plugin_symbol<T>(&self, plugin_name: &str, symbol_name: &str) -> Result<Symbol<T>, String> {
        let library = self.plugins.get(plugin_name)
            .ok_or_else(|| format!("Plugin '{}' not found. Available plugins: {:?}", plugin_name, self.plugins.keys()))?; // More informative error

        unsafe {
            library.get(symbol_name.as_bytes())
                .map_err(|e| format!("Symbol '{}' not found in plugin '{}': {}", symbol_name, plugin_name, e)) // More informative error
        }
    }

    // Optional: Method to unload a plugin - for more complete plugin management
    pub fn unload_plugin(&mut self, plugin_name: &str) -> Result<(), String> {
        if self.plugins.remove(plugin_name).is_some() {
            println!("Plugin '{}' unloaded.", plugin_name); // Optional unload message
            Ok(())
        } else {
            Err(format!("Plugin '{}' was not loaded, cannot unload.", plugin_name))
        }
    }

    // Optional: Method to list loaded plugins - useful for debugging and management
    pub fn list_plugins(&self) -> Vec<&String> {
        self.plugins.keys().collect::<Vec<&String>>()
    }
}
