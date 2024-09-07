mod args;
mod cmdline;

#[cfg(feature = "exporter")]
mod exporter;

#[cfg(feature = "pull")]
mod pull;

#[cfg(feature = "push")]
mod push;

use args::Args;
use btleplug::{api::Manager as _, platform::Manager};
use std::sync::Arc;
use tokio::{
    signal::ctrl_c,
    sync::Notify,
    task::{spawn, JoinSet},
};
use tracing as log;
use ubmsc::{CellData, Client, DeviceId, DeviceInfo, Error, Format, Options, Result};

#[cfg(feature = "exporter")]
use exporter::{Encoding, Exporter};

#[cfg(feature = "metrics")]
use ubmsc::Metrics;

#[cfg_attr(feature = "multi-thread", tokio::main)]
#[cfg_attr(not(feature = "multi-thread"), tokio::main(flavor = "current_thread"))]
async fn main() -> Result<()> {
    let args = Args::from_cmdline();

    if args.version {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        if !env!("CARGO_PKG_DESCRIPTION").is_empty() {
            println!("{}", env!("CARGO_PKG_DESCRIPTION"));
        }
        return Ok(());
    }

    #[cfg(feature = "tracing-subscriber")]
    if let Some(log) = args.log_filter() {
        use tracing_subscriber::prelude::*;

        let registry = tracing_subscriber::registry().with(log);

        #[cfg(all(feature = "stderr", feature = "journal"))]
        let registry = registry.with(if !args.journal {
            Some(tracing_subscriber::fmt::Layer::default().with_writer(std::io::stderr))
        } else {
            None
        });

        #[cfg(all(feature = "stderr", not(feature = "journal")))]
        let registry =
            registry.with(tracing_subscriber::fmt::Layer::default().with_writer(std::io::stderr));

        #[cfg(feature = "journal")]
        let registry = registry.with(if args.journal {
            Some(tracing_journald::Layer::new()?)
        } else {
            None
        });

        registry.init();
    }

    log::info!("Start...");
    log::trace!("{args:?}");

    if !args.has_action() {
        println!("Please specify the action: -i -c -e");
        return Ok(());
    }

    let mut main = Main::new(args);

    main.run().await.map_err(|error| {
        log::error!("Exit with error: {error}");
        error
    })?;

    log::info!("Stop...");

    Ok(())
}

pub struct Main {
    args: Args,
    intr: Arc<Notify>,
    clients: Vec<Client>,
}

impl core::ops::Deref for Main {
    type Target = Args;
    fn deref(&self) -> &Self::Target {
        &self.args
    }
}

impl Main {
    pub fn new(args: Args) -> Self {
        let intr = Self::intr_notify();
        let clients = Vec::default();

        Self {
            args,
            intr,
            clients,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let manager = Manager::new().await?;

        self.open_clients(&manager).await?;

        if self.has_command() {
            self.run_commands().await?;
        }

        #[cfg(feature = "pull")]
        if self.has_server() {
            self.run_exporter_server().await?;
        }

        #[cfg(feature = "push")]
        if self.has_client() {
            self.run_exporter_client().await?;
        }

        self.close_clients().await?;

        Ok(())
    }

    fn intr_notify() -> Arc<Notify> {
        let notify = Arc::new(Notify::new());

        spawn({
            let notify = notify.clone();
            async move {
                log::debug!("Avait ctrl-c signal");
                if let Err(error) = ctrl_c().await {
                    log::error!("Error while processing ctrl-c: {error}");
                }
                notify.notify_waiters();
            }
        });

        notify
    }

    async fn open_clients(&mut self, manager: &Manager) -> Result<()> {
        let options = self.client_options();

        let adapter = manager
            .adapters()
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| {
                log::error!("No Bluetooth adapters found");
                Error::NotFound
            })?;

        let found_devices: Vec<_>;

        let devices = if self.device.is_empty() {
            log::warn!("No devices passed. Scan to find all...");
            found_devices = Client::find(&adapter, &options).await?;
            &found_devices
        } else {
            &self.args.device
        };

        log::debug!("Use {} devices", devices.len());

        if devices.is_empty() {
            println!("No BMS devices found!");
            return Err(Error::NotFound);
        }

        for device_id in devices {
            let client = Client::new(&adapter, device_id, &options);
            self.clients.push(client);
        }

        Ok(())
    }

    async fn close_clients(&mut self) -> Result<()> {
        let mut joins = JoinSet::new();

        for client in self.clients.drain(..) {
            joins.spawn(async move {
                if let Err(error) = client.close().await {
                    log::error!("Error while closing client: {error}");
                }
            });
        }

        let _ = joins.join_all().await;

        Ok(())
    }
}
