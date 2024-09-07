use crate::{log, Error, Exporter, Main, Result};
use std::net::SocketAddr;

use http_body_util::Full;
use hyper::{
    body::Bytes,
    client::conn::http1::handshake,
    header::{CONTENT_TYPE, HOST},
    Request, Uri,
};
use hyper_util::rt::TokioIo;
use tokio::{net::TcpStream, select, time::interval};

#[derive(thiserror::Error, Debug)]
enum PushError {
    /// Client error
    #[error("Client error: {0}")]
    Client(#[from] Error),

    /// HttpError
    #[error("HTTP error: {0}")]
    Http(#[from] http::Error),

    /// HyperError
    #[error("Hyper error: {0}")]
    Hyper(#[from] hyper::Error),

    /// Invalid status
    #[error("Invalid response status: {0}")]
    BadStatus(u16),
}

impl From<std::io::Error> for PushError {
    fn from(error: std::io::Error) -> Self {
        Self::Client(error.into())
    }
}

impl Exporter {
    async fn do_request(
        &self,
        addr: &SocketAddr,
        url: &Uri,
    ) -> core::result::Result<(), PushError> {
        log::info!("Connect to {addr}");

        let stream = TcpStream::connect(addr).await?;

        let io = TokioIo::new(stream);

        let (mut sender, conn) = handshake(io).await?;

        tokio::task::spawn(async move {
            if let Err(error) = conn.await {
                log::error!("Connection failed: {error:?}");
            }
        });

        let mut data = Vec::with_capacity(4096);

        let content_type = self.encode(None, &mut data)?;

        let request = Request::put(url)
            .header(HOST, url.host().unwrap())
            .header(CONTENT_TYPE, content_type)
            .body(Full::<Bytes>::from(data))?;

        log::debug!("Request: {request:?}");

        let response = sender.send_request(request).await?;

        log::debug!("Response: {response:?}");

        if !response.status().is_success() {
            return Err(PushError::BadStatus(response.status().as_u16()));
        }

        Ok(())
    }
}

impl Main {
    pub async fn run_exporter_client(&self) -> Result<()> {
        let addr = self.url_addr().await?;

        let exporter = Exporter::new(self.default_encoding(), &self.clients)?;

        let mut poller = interval(self.args.scrape_interval);

        if self.exporter {
            log::info!("Start pusher for: {addr}");

            loop {
                select! {
                    _ = poller.tick() => (),
                    _ = self.intr.notified() => break,
                }

                if exporter.scrape(&self.clients).await.is_ok() {
                    if let Err(error) = exporter.do_request(&addr, &self.url).await {
                        log::error!("Error while pushing metrics: {error}");
                    }
                }
            }

            log::info!("Stop pusher for: {addr}");
        } else if exporter.scrape(&self.clients).await.is_ok() {
            if let Err(error) = exporter.do_request(&addr, &self.url).await {
                log::error!("Error while pushing metrics: {error}");
            }
        }

        Ok(())
    }
}
