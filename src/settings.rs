use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const SETTINGS_FILE: &str = "app-settings.toml";
const DEFAULT_DATABASE: &str = "inventory.sqlite3";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSettings {
    pub path: String,
    pub recent: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            database: DatabaseSettings {
                path: DEFAULT_DATABASE.to_string(),
                recent: vec![DEFAULT_DATABASE.to_string()],
            },
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        if !Path::new(SETTINGS_FILE).exists() {
            return Settings::default();
        }

        match fs::read_to_string(SETTINGS_FILE) {
            Ok(content) => match toml::from_str(&content) {
                Ok(settings) => settings,
                Err(_) => Settings::default(),
            },
            Err(_) => Settings::default(),
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let toml_str = toml::to_string_pretty(self)?;
        fs::write(SETTINGS_FILE, toml_str)?;
        Ok(())
    }

    pub fn set_database_path(&mut self, path: String) {
        self.database.path = path.clone();

        // Add to recent if not already present, keep last 10
        if !self.database.recent.contains(&path) {
            self.database.recent.insert(0, path);
            if self.database.recent.len() > 10 {
                self.database.recent.pop();
            }
        } else {
            // Move to front
            self.database.recent.retain(|p| p != &path);
            self.database.recent.insert(0, path);
        }

        let _ = self.save();
    }

    pub fn get_database_path(&self) -> &str {
        &self.database.path
    }
}
