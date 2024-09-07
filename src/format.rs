use crate::Result;
use core::fmt::Debug;
use std::io::Write;

#[cfg(feature = "serde")]
use serde::Serialize;

#[derive(Clone, Copy, Debug)]
pub enum Format {
    Rust,
    RustPretty,
    #[cfg(feature = "json")]
    Json,
    #[cfg(feature = "json")]
    JsonPretty,
    #[cfg(feature = "yaml")]
    Yaml,
    #[cfg(feature = "toml")]
    Toml,
    #[cfg(feature = "toml")]
    TomlPretty,
    #[cfg(feature = "metrics")]
    Metrics,
}

impl core::str::FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        Ok(match s {
            "r" | "rust" => Self::Rust,
            "R" | "rust-pretty" => Self::RustPretty,
            #[cfg(feature = "json")]
            "j" | "json" => Self::Json,
            #[cfg(feature = "json")]
            "J" | "json-pretty" => Self::JsonPretty,
            #[cfg(feature = "yaml")]
            "y" | "yaml" => Self::Yaml,
            #[cfg(feature = "toml")]
            "t" | "toml" => Self::Toml,
            #[cfg(feature = "toml")]
            "T" | "toml-pretty" => Self::TomlPretty,
            #[cfg(feature = "metrics")]
            "m" | "metrics" => Self::Metrics,
            _ => return Err(format!("Unknown data format: {s}")),
        })
    }
}

impl Format {
    #[cfg(not(feature = "serde"))]
    pub fn format_value<T: Debug>(&self, value: &T, output: &mut dyn Write) -> Result<()> {
        match self {
            Self::Rust => write!(output, "{value:?}")?,
            Self::RustPretty => write!(output, "{value:#?}")?,
            #[cfg(feature = "metrics")]
            Self::Metrics => {}
        }
        Ok(())
    }

    #[cfg(feature = "serde")]
    pub fn format_value<T: Debug + Serialize>(
        &self,
        value: &T,
        output: &mut dyn Write,
    ) -> Result<()> {
        match self {
            Self::Rust => write!(output, "{value:?}")?,
            Self::RustPretty => write!(output, "{value:#?}")?,
            #[cfg(feature = "json")]
            Self::Json => serde_json::to_writer(output, value)?,
            #[cfg(feature = "json")]
            Self::JsonPretty => serde_json::to_writer_pretty(output, value)?,
            #[cfg(feature = "yaml")]
            Self::Yaml => serde_yaml::to_writer(output, value)?,
            #[cfg(feature = "toml")]
            Self::Toml => write!(output, "{}", serde_toml::to_string(value)?)?,
            #[cfg(feature = "toml")]
            Self::TomlPretty => write!(output, "{}", serde_toml::to_string_pretty(value)?)?,
            #[cfg(feature = "metrics")]
            Self::Metrics => {}
        }
        Ok(())
    }
}
