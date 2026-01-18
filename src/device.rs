//
// device.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Nov 26 2022
//

use crate::config::{Config, DeviceConfig, Step};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("Device with slave_id {0} not found")]
    DeviceNotFound(u8),
    #[error("Register address {0} not found")]
    RegisterNotFound(u16),
}

/// In-memory register storage for a single device
#[derive(Debug, Clone)]
struct RegisterStorage {
    holding_registers: HashMap<u16, u16>,
    on_write_triggers: HashMap<u16, Vec<Step>>,
}

impl RegisterStorage {
    fn new(device_config: &DeviceConfig) -> Self {
        let mut holding_registers = HashMap::new();
        let mut on_write_triggers = HashMap::new();

        for reg in &device_config.holding_registers {
            holding_registers.insert(reg.address, reg.initial_value);
            if let Some(ref on_write) = reg.on_write {
                on_write_triggers.insert(reg.address, on_write.sequence.clone());
            }
        }

        Self {
            holding_registers,
            on_write_triggers,
        }
    }

    fn read_holding_register(&self, address: u16) -> Option<u16> {
        self.holding_registers.get(&address).copied()
    }

    fn write_holding_register(&mut self, address: u16, value: u16) {
        self.holding_registers.insert(address, value);
    }

    fn get_on_write_sequence(&self, address: u16) -> Option<Vec<Step>> {
        self.on_write_triggers.get(&address).cloned()
    }
}

/// Virtual device manager supporting multiple devices
pub struct DeviceManager {
    devices: HashMap<u8, Arc<RwLock<RegisterStorage>>>,
}

impl DeviceManager {
    /// Create device manager from YAML configuration
    pub fn from_config(config: Config) -> Self {
        let mut devices = HashMap::new();

        for device_config in config.devices {
            let storage = RegisterStorage::new(&device_config);
            devices.insert(device_config.slave_id, Arc::new(RwLock::new(storage)));
        }

        Self { devices }
    }

    /// Get device by slave_id
    fn get_device(&self, slave_id: u8) -> Result<Arc<RwLock<RegisterStorage>>, DeviceError> {
        self.devices
            .get(&slave_id)
            .cloned()
            .ok_or(DeviceError::DeviceNotFound(slave_id))
    }

    /// Read holding registers from a device
    pub async fn read_holding_registers(
        &self,
        slave_id: u8,
        address: u16,
        count: u16,
    ) -> Result<Vec<u16>, DeviceError> {
        let device = self.get_device(slave_id)?;
        let storage = device.read().await;

        let mut result = Vec::new();
        for i in 0..count {
            let addr = address.wrapping_add(i);
            result.push(storage.read_holding_register(addr).unwrap_or(0));
        }

        Ok(result)
    }

    /// Write holding registers to a device and trigger sequences
    pub async fn write_holding_registers(
        &self,
        slave_id: u8,
        address: u16,
        values: Vec<u16>,
    ) -> Result<(u16, u16), DeviceError> {
        let device = self.get_device(slave_id)?;
        let count = values.len() as u16;

        // Write values and collect sequences to trigger
        let sequences_to_run = {
            let mut storage = device.write().await;
            let mut sequences = Vec::new();

            for (i, &value) in values.iter().enumerate() {
                let addr = address.wrapping_add(i as u16);
                storage.write_holding_register(addr, value);

                // Check if this register has an on_write trigger
                if let Some(sequence) = storage.get_on_write_sequence(addr) {
                    sequences.push(sequence);
                }
            }

            sequences
        };

        // Spawn async tasks for each sequence
        for sequence in sequences_to_run {
            let device_clone = device.clone();
            tokio::spawn(async move {
                execute_sequence(device_clone, sequence).await;
            });
        }

        Ok((address, count))
    }

    // Placeholder methods for other Modbus functions
    // Note: These return errors for invalid slave_ids as they are not implemented in the config
    pub async fn read_input_registers(
        &self,
        slave_id: u8,
        _address: u16,
        count: u16,
    ) -> Result<Vec<u16>, DeviceError> {
        // Verify device exists even though we don't use it
        let _ = self.get_device(slave_id)?;
        // Return zeros for input registers (not implemented in config)
        Ok(vec![0; count as usize])
    }

    pub async fn read_discrete_inputs(
        &self,
        slave_id: u8,
        _address: u16,
        count: u16,
    ) -> Result<Vec<bool>, DeviceError> {
        // Verify device exists even though we don't use it
        let _ = self.get_device(slave_id)?;
        // Return false for discrete inputs (not implemented in config)
        Ok(vec![false; count as usize])
    }

    pub async fn read_coils(
        &self,
        slave_id: u8,
        _address: u16,
        count: u16,
    ) -> Result<Vec<bool>, DeviceError> {
        // Verify device exists even though we don't use it
        let _ = self.get_device(slave_id)?;
        // Return false for coils (not implemented in config)
        Ok(vec![false; count as usize])
    }

    pub async fn write_coils(
        &self,
        slave_id: u8,
        address: u16,
        values: Vec<bool>,
    ) -> Result<(u16, u16), DeviceError> {
        // Verify device exists even though we don't use it
        let _ = self.get_device(slave_id)?;
        // Not implemented in config, just return success
        Ok((address, values.len() as u16))
    }
}

/// Execute a sequence of steps asynchronously
async fn execute_sequence(device: Arc<RwLock<RegisterStorage>>, steps: Vec<Step>) {
    for step in steps {
        match step {
            Step::Set { address, value } => {
                let mut storage = device.write().await;
                storage.write_holding_register(address, value);
            }
            Step::ForMs {
                address,
                value,
                duration_ms,
            } => {
                // Set the value
                {
                    let mut storage = device.write().await;
                    storage.write_holding_register(address, value);
                }
                // Wait for the duration
                tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
            }
            Step::WaitMs { duration_ms } => {
                tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[tokio::test]
    async fn test_device_manager_from_config() {
        let yaml = r#"
devices:
  - slave_id: 1
    holding_registers:
      - address: 0
        initial_value: 100
      - address: 1
        initial_value: 200
"#;
        let config = Config::from_yaml(yaml).unwrap();
        let manager = DeviceManager::from_config(config);

        let regs = manager.read_holding_registers(1, 0, 2).await.unwrap();
        assert_eq!(regs, vec![100, 200]);
    }

    #[tokio::test]
    async fn test_write_holding_registers() {
        let yaml = r#"
devices:
  - slave_id: 1
    holding_registers:
      - address: 0
        initial_value: 0
"#;
        let config = Config::from_yaml(yaml).unwrap();
        let manager = DeviceManager::from_config(config);

        manager
            .write_holding_registers(1, 0, vec![42])
            .await
            .unwrap();

        let regs = manager.read_holding_registers(1, 0, 1).await.unwrap();
        assert_eq!(regs, vec![42]);
    }

    #[tokio::test]
    async fn test_sequence_execution() {
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
              duration_ms: 10
            - action: set
              address: 2
              value: 200
      - address: 1
        initial_value: 0
      - address: 2
        initial_value: 0
"#;
        let config = Config::from_yaml(yaml).unwrap();
        let manager = DeviceManager::from_config(config);

        // Write to address 0 to trigger sequence
        manager
            .write_holding_registers(1, 0, vec![1])
            .await
            .unwrap();

        // Give sequence time to execute
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Check that sequence executed
        let regs = manager.read_holding_registers(1, 1, 2).await.unwrap();
        assert_eq!(regs, vec![100, 200]);
    }

    #[tokio::test]
    async fn test_multiple_devices() {
        let yaml = r#"
devices:
  - slave_id: 1
    holding_registers:
      - address: 0
        initial_value: 100
  - slave_id: 2
    holding_registers:
      - address: 0
        initial_value: 200
"#;
        let config = Config::from_yaml(yaml).unwrap();
        let manager = DeviceManager::from_config(config);

        let regs1 = manager.read_holding_registers(1, 0, 1).await.unwrap();
        let regs2 = manager.read_holding_registers(2, 0, 1).await.unwrap();

        assert_eq!(regs1, vec![100]);
        assert_eq!(regs2, vec![200]);
    }

    #[tokio::test]
    async fn test_device_not_found() {
        let yaml = r#"
devices:
  - slave_id: 1
    holding_registers: []
"#;
        let config = Config::from_yaml(yaml).unwrap();
        let manager = DeviceManager::from_config(config);

        let result = manager.read_holding_registers(99, 0, 1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_input_registers_device_not_found() {
        let yaml = r#"
devices:
  - slave_id: 1
    holding_registers: []
"#;
        let config = Config::from_yaml(yaml).unwrap();
        let manager = DeviceManager::from_config(config);

        // Should return error for non-existent device
        let result = manager.read_input_registers(99, 0, 1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_coils_device_not_found() {
        let yaml = r#"
devices:
  - slave_id: 1
    holding_registers: []
"#;
        let config = Config::from_yaml(yaml).unwrap();
        let manager = DeviceManager::from_config(config);

        // Should return error for non-existent device
        let result = manager.read_coils(99, 0, 1).await;
        assert!(result.is_err());
        
        let result = manager.write_coils(99, 0, vec![true]).await;
        assert!(result.is_err());
    }
}
