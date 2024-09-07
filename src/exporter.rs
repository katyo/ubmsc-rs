use crate::{log, Client, Metrics, Result};
use prometheus::{Encoder, ProtobufEncoder, Registry, TextEncoder};
use std::io::Write;

#[derive(Clone, Copy, Default, Debug)]
pub enum Encoding {
    #[default]
    Text,
    Protobuf,
}

impl Encoding {
    pub fn from_accept(accept: impl AsRef<[u8]>) -> Option<Self> {
        let accept = accept.as_ref();
        if find_seq(accept, b"application/vnd.google.protobuf").is_some() {
            Some(Self::Protobuf)
        } else if find_seq(accept, b"text/plain").is_some() {
            Some(Self::Text)
        } else {
            None
        }
    }
}

pub struct Exporter {
    registry: Registry,
    text_encoder: TextEncoder,
    protobuf_encoder: ProtobufEncoder,
    metrics: Vec<Metrics>,
    default_encoding: Encoding,
}

impl<'x> Exporter {
    pub fn new(default_encoding: Encoding, clients: &[Client]) -> Result<Self> {
        let registry = Registry::new();
        let metrics = Vec::default();
        let text_encoder = TextEncoder::new();
        let protobuf_encoder = ProtobufEncoder::new();

        let mut this = Self {
            registry,
            text_encoder,
            protobuf_encoder,
            metrics,
            default_encoding,
        };

        this.metrics(clients)?;

        Ok(this)
    }

    fn metrics(&mut self, clients: &[Client]) -> Result<()> {
        for client in clients {
            let metric = Metrics::new(client.device_id())?;
            metric.register(Some(&self.registry))?;
            self.metrics.push(metric);
        }
        Ok(())
    }

    pub async fn scrape(&self, clients: &[Client]) -> Result<()> {
        for (client, metrics) in clients.iter().zip(self.metrics.iter()) {
            let device_id = client.device_id();
            log::info!("Scrape metrics from: '{device_id}'");

            if let Err(error) = client.open().await {
                log::error!("Error while connecting: {error}");
            } else {
                match client.device_info().await {
                    Ok(device_info) => metrics.scrape(&device_info),
                    Err(error) => {
                        log::error!("Error while fetch device info from '{device_id}': {error}")
                    }
                }
                match client.cell_data().await {
                    Ok(cell_data) => metrics.scrape(&cell_data),
                    Err(error) => {
                        log::error!("Error while fetch cell data from '{device_id}': {error}")
                    }
                }
                if let Err(error) = client.close().await {
                    log::error!("Error while disconnecting: {error}");
                }
            }
        }
        Ok(())
    }

    pub fn encode(&self, encoding: Option<Encoding>, mut output: impl Write) -> Result<&str> {
        Ok(match encoding.unwrap_or(self.default_encoding) {
            Encoding::Protobuf => {
                self.protobuf_encoder
                    .encode(&self.registry.gather(), &mut output)?;
                self.protobuf_encoder.format_type()
            }
            Encoding::Text => {
                self.text_encoder
                    .encode(&self.registry.gather(), &mut output)?;
                self.text_encoder.format_type()
            }
        })
    }
}

fn find_seq<T>(seq: &[T], sub: &[T]) -> Option<usize>
where
    for<'a> &'a [T]: PartialEq,
{
    seq.windows(sub.len()).position(|win| win == sub)
}
