use crate::{log, CellData, DeviceInfo, Format, Main, Result};

#[cfg(feature = "metrics")]
use crate::Metrics;

#[cfg(feature = "metrics")]
use prometheus::{Encoder, Registry, TextEncoder};

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Outputs {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Vec::is_empty"))]
    pub device_info: Vec<DeviceInfo>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Vec::is_empty"))]
    pub cell_data: Vec<CellData>,
}

impl Main {
    pub async fn run_commands(&self) -> Result<()> {
        let mut outputs = Outputs::default();

        #[cfg(feature = "metrics")]
        let registry = Registry::new();

        for client in &self.clients {
            #[cfg(feature = "metrics")]
            let metrics = Metrics::new(client.device_id())?;
            #[cfg(feature = "metrics")]
            metrics.register(Some(&registry))?;

            let device_id = client.device_id();

            log::info!("Connect to: '{device_id}'");

            if let Err(error) = client.open().await {
                log::error!("Error while connecting to device: {error}");
            } else {
                if self.device_info {
                    match client.device_info().await {
                        Ok(device_info) => {
                            #[cfg(feature = "metrics")]
                            if matches!(self.format, Format::Metrics) {
                                metrics.scrape(&device_info);
                            }
                            outputs.device_info.push(device_info);
                        }
                        Err(error) => log::error!("Error while fetching device info: {error}"),
                    }
                }

                if self.cell_data {
                    match client.cell_data().await {
                        Ok(cell_data) => {
                            #[cfg(feature = "metrics")]
                            if matches!(self.format, Format::Metrics) {
                                metrics.scrape(&cell_data);
                            }
                            outputs.cell_data.push(cell_data);
                        }
                        Err(error) => log::error!("Error while fetching cell data: {error}"),
                    }
                }

                log::info!("Disconnect from: '{device_id}'");

                if let Err(error) = client.close().await {
                    log::error!("Error while disconnecting from device: {error}");
                }
            }
        }

        let mut output = std::io::stdout();
        self.format.format_value(&outputs, &mut output)?;

        #[cfg(feature = "metrics")]
        if matches!(self.format, Format::Metrics) {
            let encoder = TextEncoder::new();
            encoder.encode(&registry.gather(), &mut output)?;
        }

        Ok(())
    }
}
