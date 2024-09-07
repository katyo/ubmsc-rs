/// Common result type
pub type Result<T> = core::result::Result<T, Error>;

/// Common error type
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Input/output error
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),
    /// Bluetooth error
    #[error("Bluetooth: {0}")]
    Bt(#[from] btleplug::Error),
    /// Prometheus error
    #[cfg(feature = "metrics")]
    #[error("Prometheus error: {0}")]
    Prometheus(#[from] prometheus::Error),
    /// UTF-8 decoding error
    #[error("UTF-8 error")]
    Utf8(#[from] core::str::Utf8Error),
    /// Timeout reached
    #[error("Timeout")]
    Timeout,
    /// Not found error
    #[error("Not found")]
    NotFound,
    /// Invalid checksum
    #[error("Invalid checksum")]
    BadCrc,
    /// Invalid record type
    #[error("Invalid record type")]
    BadRecordType,
    /// Connection lost
    #[error("Connection lost")]
    LostConnection,
    /// Not enough data
    #[error("Not enough data")]
    NotEnoughData,
    /// Not supported
    #[error("Not supported")]
    NotSupported,
    /// Unable to resolve hostname
    #[error("Unknown hostname")]
    UnknownHostname,
    /// Json format error
    #[cfg(feature = "json")]
    #[error("JSON format error: {0}")]
    JsonEnc(#[from] serde_json::Error),
    /// Yaml format error
    #[cfg(feature = "yaml")]
    #[error("YAML format error: {0}")]
    YamlEnc(#[from] serde_yaml::Error),
    /// Toml format error
    #[cfg(feature = "toml")]
    #[error("TOML format error: {0}")]
    TomlEnc(#[from] serde_toml::ser::Error),
}

impl From<tokio::time::error::Elapsed> for Error {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        Self::Timeout
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(error: std::string::FromUtf8Error) -> Self {
        error.utf8_error().into()
    }
}
