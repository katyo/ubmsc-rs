use crate::{DeviceId, Format, Options};
use argp::FromArgs;
use core::time::Duration;

#[cfg(feature = "exporter")]
use crate::Encoding;
#[cfg(feature = "exporter")]
use hyper::Uri;
#[cfg(feature = "exporter")]
use std::net::{IpAddr, SocketAddr};

#[cfg(feature = "tracing-subscriber")]
use tracing_subscriber::EnvFilter;

/// Battery Management Systems (BMS) interface.
#[cfg_attr(feature = "push", doc = "")]
#[cfg_attr(
    feature = "push",
    doc = "When passed both -e and -p options push client will be run in continuous mode with specified interval."
)]
#[derive(FromArgs, Debug)]
pub struct Args {
    /// Show version and exit
    #[argp(switch, short = 'v')]
    pub version: bool,

    /// Logging filter (example: jk_bms=debug)
    #[cfg(feature = "tracing-subscriber")]
    #[argp(
        option,
        short = 'l',
        arg_name = "filter",
        from_str_fn(Args::parse_env_filter)
    )]
    pub log: Option<EnvFilter>,

    /// Enable log to journald (log to stderr by default)
    #[cfg(feature = "journal")]
    #[argp(switch, short = 'j')]
    pub journal: bool,

    /// Bluetooth scanning timeout in seconds (30 by default)
    #[argp(
        option,
        short = 't',
        arg_name = "seconds",
        default = "Duration::from_secs(30)",
        from_str_fn(Args::parse_duration)
    )]
    pub scan_timeout: Duration,

    /// Bluetooth request timeout in seconds (5 by default)
    #[argp(
        option,
        short = 'r',
        arg_name = "seconds",
        default = "Duration::from_secs(5)",
        from_str_fn(Args::parse_duration)
    )]
    pub request_timeout: Duration,

    /// Device addresses or names (will try to scan if nothing passed)
    #[argp(
        option,
        short = 'd',
        arg_name = "address",
        from_str_fn(Args::parse_device_id)
    )]
    pub device: Vec<DeviceId>,

    /// Data format: rust(r) (by default) rust-pretty(R)
    #[cfg_attr(feature = "json", doc = "json(j) json-pretty(J)")]
    #[cfg_attr(feature = "yaml", doc = "yaml(y)")]
    #[cfg_attr(feature = "toml", doc = "toml(t) toml-pretty(T)")]
    #[cfg_attr(feature = "metrics", doc = "metrics(m)")]
    #[argp(
        option,
        short = 'f',
        arg_name = "format",
        default = "Format::Rust",
        from_str_fn(core::str::FromStr::from_str)
    )]
    pub format: Format,

    /// Show device info
    #[argp(switch, short = 'i')]
    pub device_info: bool,

    /// Show cell data
    #[argp(switch, short = 'c')]
    pub cell_data: bool,

    /// Run prometheus exporter
    #[cfg(feature = "exporter")]
    #[argp(switch, short = 'e')]
    pub exporter: bool,

    /// Run prometheus push gateway client
    #[cfg(feature = "push")]
    #[argp(switch, short = 'p')]
    pub push: bool,

    /// Prometheus exporter URL to listen/connect
    #[cfg(feature = "exporter")]
    #[argp(
        option,
        short = 'u',
        default = "\"http://127.0.0.1:9889/metrics\".parse().unwrap()",
        from_str_fn(Args::parse_url)
    )]
    pub url: Uri,

    /// Metrics scraping interval (60s by default)
    #[cfg(feature = "exporter")]
    #[argp(
        option,
        short = 's',
        arg_name = "seconds",
        default = "Duration::from_secs(60)",
        from_str_fn(Args::parse_duration)
    )]
    pub scrape_interval: Duration,

    /// Prefer protobuf data format
    #[cfg(feature = "exporter")]
    #[argp(switch, short = 'b')]
    pub protobuf: bool,
}

impl Args {
    /// Create args from command-line
    pub fn from_cmdline() -> Self {
        argp::parse_args_or_exit(argp::DEFAULT)
    }

    /// Get log filter
    #[cfg(feature = "tracing-subscriber")]
    pub fn log_filter(&self) -> Option<EnvFilter> {
        self.log
            .as_ref()
            .and_then(|log| log.to_string().parse().ok())
    }

    /// Need to exec command
    pub fn has_command(&self) -> bool {
        self.device_info || self.cell_data
    }

    /// Need run exporter server
    #[cfg(feature = "pull")]
    pub fn has_server(&self) -> bool {
        #[cfg(not(feature = "push"))]
        {
            self.exporter
        }

        #[cfg(feature = "push")]
        {
            self.exporter && !self.push
        }
    }

    /// Need run exporter client
    #[cfg(feature = "push")]
    pub fn has_client(&self) -> bool {
        self.push
    }

    /// Need to do some action
    pub fn has_action(&self) -> bool {
        #[cfg(all(not(feature = "pull"), not(feature = "push")))]
        {
            self.has_command()
        }

        #[cfg(all(feature = "pull", not(feature = "push")))]
        {
            self.has_command() || self.has_server()
        }

        #[cfg(all(not(feature = "pull"), feature = "push"))]
        {
            self.has_command() || self.has_client()
        }

        #[cfg(all(feature = "pull", feature = "push"))]
        {
            self.has_command() || self.has_server() || self.has_client()
        }
    }

    #[cfg(feature = "exporter")]
    pub async fn url_addr(&self) -> crate::Result<SocketAddr> {
        let host = self.url.host().unwrap_or("127.0.0.1");
        let port = self.url.port_u16().unwrap_or_else(|| {
            if self
                .url
                .scheme()
                .map(|scheme| scheme == &http::uri::Scheme::HTTPS)
                .unwrap_or_default()
            {
                443
            } else {
                80
            }
        });

        Ok(if let Ok(addr) = host.parse::<IpAddr>() {
            SocketAddr::new(addr, port)
        } else {
            tokio::net::lookup_host(host)
                .await?
                .next()
                .ok_or(crate::Error::UnknownHostname)?
        })
    }

    #[cfg(feature = "exporter")]
    pub fn default_encoding(&self) -> Encoding {
        if self.protobuf {
            Encoding::Protobuf
        } else {
            Encoding::Text
        }
    }

    /// Client options
    pub fn client_options(&self) -> Options {
        Options {
            scan_timeout: self.scan_timeout,
            request_timeout: self.request_timeout,
        }
    }

    #[cfg(feature = "exporter")]
    fn parse_url(s: &str) -> Result<Uri, String> {
        s.parse::<Uri>()
            .map_err(|error| error.to_string())
            .and_then(|url| {
                if url
                    .scheme_str()
                    .map(|scheme| scheme != "http" && scheme != "https")
                    .unwrap_or_default()
                {
                    return Err("Only HTTP(s) protocol is supported".to_string());
                }
                Ok(url)
            })
    }

    fn parse_duration(s: &str) -> Result<Duration, String> {
        s.parse::<u32>()
            .map(|seconds| Duration::from_secs(seconds as _))
            .map_err(|error| format!("Bad timeout value: {error}"))
    }

    fn parse_device_id(s: &str) -> Result<DeviceId, String> {
        Ok(s.parse().unwrap())
    }

    #[cfg(feature = "tracing-subscriber")]
    fn parse_env_filter(s: &str) -> Result<EnvFilter, String> {
        s.parse()
            .map_err(|error| format!("Bad tracing filter: {error}"))
    }
}
