use crate::{Error, MacAddr, Result};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Device identifier
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DeviceId {
    /// MAC Address
    Mac(MacAddr),
    /// Device name
    Name(String),
}

impl core::str::FromStr for DeviceId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(s.parse()
            .map(Self::Mac)
            .unwrap_or_else(|_| Self::Name(s.into())))
    }
}

impl From<MacAddr> for DeviceId {
    fn from(mac: MacAddr) -> Self {
        Self::Mac(mac)
    }
}

impl From<&'_ str> for DeviceId {
    fn from(name: &'_ str) -> Self {
        Self::Name(name.into())
    }
}

impl From<String> for DeviceId {
    fn from(name: String) -> Self {
        Self::Name(name)
    }
}

impl core::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Self::Mac(mac) => mac.fmt(f),
            Self::Name(name) => name.fmt(f),
        }
    }
}

/// BMS device information
#[derive(Clone, Default, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DeviceInfo {
    /// Model name
    pub device_model: String,
    /// Hadrware version
    pub hardware_version: String,
    /// Firmware version
    pub software_version: String,
    /// Time in seconds since poweron
    pub up_time: usize,
    /// Number of powerons
    pub poweron_times: usize,
    /// Device name
    pub device_name: String,
    /// Device passcode
    pub device_passcode: String,
    /// Manufacturing date
    pub manufacturing_date: String,
    /// Serial number
    pub serial_number: String,
    /// Passcode
    pub passcode: String,
    /// Userdata
    pub userdata: String,
    /// Passcode to change settings
    pub setup_passcode: String,
    /// Second userdata
    pub userdata2: String,
}

/// BMS cell data
#[derive(Clone, Default, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CellData {
    /// Cell voltages in Volts
    pub cell_voltage: Vec<f32>,
    /// Averate cell voltage in Volts
    pub average_cell_voltage: f32,
    /// Maximum voltage difference between cells in Volts
    pub delta_cell_voltage: f32,
    /// Balance current in Amperes
    pub balance_current: f32,
    /// Cell resistances in Ohms
    pub cell_resistance: Vec<f32>,
    /// Amount battery voltage between terminals in Volts
    pub battery_voltage: f32,
    /// Amount battery power in Watts
    pub battery_power: f32,
    /// Battery current in Amperes
    pub battery_current: f32,
    /// Battery temperatures in Celsius degrees
    pub battery_temperature: Vec<f32>,
    /// BMS power mosfet temperature in Celsius degrees
    pub mosfet_temperature: f32,
    /// Remain battery capacity in percents
    pub remain_percent: u8,
    /// Remain battery capacity in Amperes*Hours
    pub remain_capacity: f32,
    /// Nominal battery capacity in Amperes*Hours
    pub nominal_capacity: f32,
    /// Number of battery cycles
    pub cycle_count: usize,
    /// Cicle battery capacity in Amperes*Hours
    pub cycle_capacity: f32,
    /// Time in seconds since last poweron
    pub up_time: usize,
}
