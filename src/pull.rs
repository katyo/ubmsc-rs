use crate::{log, Encoding, Exporter, Main, Result};
use std::sync::Arc;

use core::time::Duration;
use http_body_util::Full;
use hyper::{
    body::{Bytes, Incoming},
    header::{ACCEPT, CONTENT_TYPE},
    server::conn::http1,
    service::service_fn,
    Request, Response,
};
use hyper_util::rt::TokioIo;
use tokio::{
    net::TcpListener,
    select,
    task::JoinSet,
    time::{interval, timeout},
};

impl Exporter {
    async fn serve_request(
        &self,
        request: Request<Incoming>,
    ) -> hyper::Result<Response<Full<Bytes>>> {
        if request.method() != "GET" {
            return Ok(Response::builder()
                .status(405)
                .header(CONTENT_TYPE, "text/plain")
                .body(Full::new(Bytes::from("Method not allowed")))
                .unwrap());
        }

        if request.uri() != "/metrics" {
            return Ok(Response::builder()
                .status(404)
                .header(CONTENT_TYPE, "text/plain")
                .body(Full::new(Bytes::from("Not found")))
                .unwrap());
        }

        let mut buffer = Vec::with_capacity(4096);

        let encoding = request
            .headers()
            .get(ACCEPT)
            .and_then(Encoding::from_accept);

        let content_type = match self.encode(encoding, &mut buffer) {
            Ok(content_type) => content_type,
            Err(error) => {
                log::error!("Error while encoding metrics: {error}");

                return Ok(Response::builder()
                    .status(200)
                    .header(CONTENT_TYPE, "text/plain")
                    .body(Full::new(Bytes::from("Internal error")))
                    .unwrap());
            }
        };

        let response = Response::builder()
            .status(200)
            .header(CONTENT_TYPE, content_type)
            .body(Full::new(Bytes::from(buffer)))
            .unwrap();

        Ok(response)
    }
}

impl Main {
    pub async fn run_exporter_server(&self) -> Result<()> {
        let exporter = Arc::new(Exporter::new(self.default_encoding(), &self.clients)?);

        let addr = self.url_addr().await?;

        log::info!("Start server at: {addr}");

        let listener = TcpListener::bind(addr).await?;

        let server = tokio::task::spawn({
            let exporter = exporter.clone();
            let intr = self.intr.clone();
            async move {
                let mut joins = JoinSet::new();
                loop {
                    let stream = match select! {
                        acception = listener.accept() => acception,
                        _ = intr.notified() => break,
                    } {
                        Ok((stream, _)) => stream,
                        Err(error) => {
                            log::error!("Error while accepting incoming connection: {error}");
                            break;
                        }
                    };

                    let io = TokioIo::new(stream);

                    let exporter = exporter.clone();

                    joins.spawn(async move {
                        if let Err(err) = http1::Builder::new()
                            .serve_connection(
                                io,
                                service_fn(|request| async {
                                    log::debug!("Process request: {request:?}");

                                    exporter.serve_request(request).await.map(|response| {
                                        log::debug!("Send response: {response:?}");
                                        response
                                    })
                                }),
                            )
                            .await
                        {
                            log::error!("Error serving connection: {:?}", err);
                        }
                    });
                }

                log::info!("Await closing connections");
                let _ = timeout(Duration::from_secs(5), joins.join_all()).await;

                log::info!("Stop server at: {addr}");
            }
        });

        let mut poller = interval(self.args.scrape_interval);

        log::info!("Start scraper");

        loop {
            select! {
                _ = poller.tick() => (),
                _ = self.intr.notified() => break,
            }

            // Ignore errors
            let _ = exporter.scrape(&self.clients).await;
        }

        log::info!("Stop scraper");

        if let Err(error) = server.await {
            log::error!("Error in server task: {error}");
        }

        Ok(())
    }
}
