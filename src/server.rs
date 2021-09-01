use async_trait::async_trait;
use chrono::Utc;
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, UdpSocket},
    sync::broadcast,
};

pub type Shutdown = broadcast::Receiver<SignalName>;

#[derive(Debug, Clone, Copy)]
pub enum SignalName {
    SigInt,
    SigTerm,
}

#[async_trait]
pub trait Server {
    async fn run(self, shutdown: Shutdown);
}

#[async_trait]
impl Server for TcpListener {
    #[tracing::instrument(skip(self, shutdown), fields(proto = "tcp"))]
    async fn run(self, mut shutdown: Shutdown) {
        loop {
            tokio::select! {
                conn = self.accept() => match conn {
                    Ok((mut stream, addr)) => {
                        tracing::info!(?addr, "connection accepted");
                        let daytime = Utc::now();
                        if let Err(err) = stream.write_all(daytime.to_rfc2822().as_bytes()).await {
                            tracing::error!(?err, "error sending daytime");
                        }
                    },
                    Err(err) => tracing::error!(?err, "could not accept connection"),
                },
                sig = shutdown.recv() => {
                         tracing::info!(?sig, "caught signal, shutting down");
                         break;
                     }
            }
        }
    }
}

#[async_trait]
impl Server for UdpSocket {
    #[tracing::instrument(skip(self, shutdown), fields(proto = "udp"))]
    async fn run(self, mut shutdown: Shutdown) {
        let mut buf = [0_u8; 2];
        loop {
            tokio::select! {
                conn = self.recv_from(&mut buf) => match conn {
                    Ok((_, addr)) => {
                        tracing::info!(?addr, "connection accepted");
                        let daytime = Utc::now();
                        if let Err(err) = self.send_to(daytime.to_rfc2822().as_bytes(), addr).await {
                            tracing::error!(?err, "error sending daytime");
                        }
                    },
                    Err(err) => tracing::error!(?err, "could not accept connection"),
                },
                sig = shutdown.recv() => {
                         tracing::info!(?sig, "caught signal, shutting down");
                         break;
                     }
            }
        }
    }
}
