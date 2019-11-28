use glutin::dpi::LogicalPosition;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;

#[derive(Serialize, Deserialize)]
pub struct PersistWindowState {
    pub monitor_name: Option<String>,
    pub logical_position: LogicalPosition,
}

impl PersistWindowState {
    pub fn save(&self) {
        match serde_yaml::to_string(self) {
            Ok(config_string) => {
                fs::write(Self::config_filename(), config_string).unwrap();
            }
            Err(e) => {
                println!("Error saving config to string: {:?}", e);
            }
        }
    }

    pub fn restore() -> Self {
        match fs::File::open(Self::config_filename()) {
            Ok(mut f) => {
                let mut config = String::new();
                match f.read_to_string(&mut config) {
                    Ok(_) => match serde_yaml::from_str::<Self>(&config) {
                        Ok(persisted) => return persisted,
                        Err(e) => println!("Error de-serializing config: {:?}", e),
                    },
                    Err(e) => println!("Error reading config file: {:?}", e),
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => println!("Error opening config file: {:?}", e),
        }
        Self::default()
    }

    fn config_filename() -> String {
        String::from(".bim_persist_state.yaml")
    }
}

impl Default for PersistWindowState {
    fn default() -> Self {
        Self {
            logical_position: LogicalPosition::new(400.0, 50.0),
            monitor_name: None,
        }
    }
}
