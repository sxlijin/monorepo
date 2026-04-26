use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, Host};
use std::fmt;

#[derive(Debug, Clone)]
pub struct AudioDevice {
    pub name: String,
    pub is_default: bool,
}

impl fmt::Display for AudioDevice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_default {
            write!(f, "{} (Default)", self.name)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

pub struct DeviceManager {
    pub host: Host,
}

impl DeviceManager {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        Ok(DeviceManager { host })
    }

    pub fn list_output_devices(&self) -> Result<Vec<AudioDevice>> {
        let mut devices = Vec::new();

        // Get default device name for comparison
        let default_device = self.host.default_output_device();
        let default_name = default_device.as_ref().and_then(|d| d.name().ok());

        // List all output devices
        for device in self.host.output_devices()? {
            if let Ok(name) = device.name() {
                let is_default = default_name
                    .as_ref()
                    .map_or(false, |default| default == &name);
                devices.push(AudioDevice { name, is_default });
            }
        }

        Ok(devices)
    }

    pub fn get_default_output_device(&self) -> Option<Device> {
        self.host.default_output_device()
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new().expect("Failed to create DeviceManager")
    }
}
