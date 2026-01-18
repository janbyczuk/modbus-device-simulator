//
// config.rs
//
// YAML configuration structures for modbus device simulator
//

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Invalid slave_id: {0}. Must be in range 1..=254")]
    InvalidSlaveId(u8),
    #[error("Duplicate holding register address {0} for device {1}")]
    DuplicateAddress(u16, u8),
    #[error("Empty sequence in on_write for register {0}")]
    EmptySequence(u16),
    #[error("YAML parsing error: {0}")]
    YamlError(#[from] serde_yaml::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub devices: Vec<DeviceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub slave_id: u8,
    #[serde(default)]
    pub holding_registers: Vec<HoldingRegister>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoldingRegister {
    pub address: u16,
    #[serde(default)]
    pub initial_value: u16,
    #[serde(default)]
    pub on_write: Option<OnWrite>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnWrite {
    pub sequence: Vec<Step>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum Step {
    #[serde(rename = "set")]
    Set { address: u16, value: u16 },
    #[serde(rename = "for_ms")]
    ForMs { address: u16, value: u16, duration_ms: u64 },
    #[serde(rename = "wait_ms")]
    WaitMs { duration_ms: u64 },
}

impl Config {
    /// Parse YAML configuration from string
    pub fn from_yaml(yaml: &str) -> Result<Self, ConfigError> {
        let config: Config = serde_yaml::from_str(yaml)?;
        config.validate()?;
        Ok(config)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        for device in &self.devices {
            // Validate slave_id range
            if device.slave_id == 0 || device.slave_id > 254 {
                return Err(ConfigError::InvalidSlaveId(device.slave_id));
            }

            // Check for duplicate holding register addresses
            let mut addresses = HashMap::new();
            for reg in &device.holding_registers {
                if addresses.contains_key(&reg.address) {
                    return Err(ConfigError::DuplicateAddress(reg.address, device.slave_id));
                }
                addresses.insert(reg.address, true);

                // Validate on_write sequences are not empty
                if let Some(ref on_write) = reg.on_write {
                    if on_write.sequence.is_empty() {
                        return Err(ConfigError::EmptySequence(reg.address));
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_config() {
        let yaml = r#"
devices:
  - slave_id: 1
    holding_registers:
      - address: 0
        initial_value: 100
      - address: 1
        initial_value: 200
"#;
        let config = Config::from_yaml(yaml);
        assert!(config.is_ok());
    }

    #[test]
    fn test_invalid_slave_id_zero() {
        let yaml = r#"
devices:
  - slave_id: 0
    holding_registers: []
"#;
        let config = Config::from_yaml(yaml);
        assert!(config.is_err());
    }

    #[test]
    fn test_invalid_slave_id_too_high() {
        let yaml = r#"
devices:
  - slave_id: 255
    holding_registers: []
"#;
        let config = Config::from_yaml(yaml);
        assert!(config.is_err());
    }

    #[test]
    fn test_duplicate_addresses() {
        let yaml = r#"
devices:
  - slave_id: 1
    holding_registers:
      - address: 0
        initial_value: 100
      - address: 0
        initial_value: 200
"#;
        let config = Config::from_yaml(yaml);
        assert!(config.is_err());
    }

    #[test]
    fn test_empty_sequence() {
        let yaml = r#"
devices:
  - slave_id: 1
    holding_registers:
      - address: 0
        initial_value: 100
        on_write:
          sequence: []
"#;
        let config = Config::from_yaml(yaml);
        assert!(config.is_err());
    }

    #[test]
    fn test_with_sequence() {
        let yaml = r#"
devices:
  - slave_id: 1
    holding_registers:
      - address: 0
        initial_value: 0
        on_write:
          sequence:
            - action: set
              address: 1
              value: 100
            - action: wait_ms
              duration_ms: 1000
            - action: for_ms
              address: 1
              value: 200
              duration_ms: 2000
"#;
        let config = Config::from_yaml(yaml);
        assert!(config.is_ok());
    }
}
