use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, CosmicConfigEntry)]
pub struct Config {
    pub tailscale_exit_nodes: Vec<String>,
    pub surfshark_locations: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tailscale_exit_nodes: vec![
                "None (Direct)".to_string(),
                "cloud-pc".to_string(),
                "docker-host".to_string(),
                "docker-host-tertiary".to_string(),
            ],
            surfshark_locations: vec![
                "Delhi".to_string(),
                "London".to_string(),
                "Chicago".to_string(),
            ],
        }
    }
}

impl Config {
    pub const VERSION: u64 = 1;
}