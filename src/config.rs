use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const DEFAULT_HOTKEY: &str = "Ctrl+Shift+F8";
pub const DEFAULT_HOTKEY_REPEAT: i32 = 0;
pub const DEFAULT_INDICATOR_DURATION: i32 = 1000;
pub const DEFAULT_INDICATOR_SIZE: i32 = 240;
pub const DEFAULT_INDICATOR_TRANSPARENCY: i32 = 200;
pub const DEFAULT_INDICATOR_FOREGROUND_TRANSPARENCY: i32 = 255;

#[derive(Clone, Debug)]
pub struct Config {
    path: PathBuf,
    values: BTreeMap<String, String>,
}

impl Config {
    pub fn load_or_create(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut config = Self {
            path,
            values: BTreeMap::new(),
        };
        config.load();
        config.set_default("hotkey", DEFAULT_HOTKEY);
        config.set_default("hotkeyRepeat", DEFAULT_HOTKEY_REPEAT.to_string());
        config.set_default("indicatorDuration", DEFAULT_INDICATOR_DURATION.to_string());
        config.set_default("indicatorSize", DEFAULT_INDICATOR_SIZE.to_string());
        config.set_default(
            "indicatorTransparency",
            DEFAULT_INDICATOR_TRANSPARENCY.to_string(),
        );
        config.set_default(
            "indicatorForegroundTransparency",
            DEFAULT_INDICATOR_FOREGROUND_TRANSPARENCY.to_string(),
        );
        config.save()?;
        Ok(config)
    }

    fn load(&mut self) {
        let Ok(contents) = fs::read_to_string(&self.path) else {
            return;
        };

        for line in contents.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            if key.is_empty() {
                continue;
            }
            self.values
                .insert(key.to_string(), value.trim_start().to_string());
        }
    }

    pub fn save(&self) -> io::Result<()> {
        let mut contents = String::new();
        for (key, value) in &self.values {
            contents.push_str(key);
            contents.push_str(" = ");
            contents.push_str(value);
            contents.push('\n');
        }
        fs::write(&self.path, contents)
    }

    pub fn str_value(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub fn int_value(&self, key: &str) -> Option<i32> {
        self.str_value(key)?.parse().ok()
    }

    fn set_default(&mut self, key: &str, value: impl Into<String>) {
        self.values
            .entry(key.to_string())
            .or_insert_with(|| value.into());
    }
}
