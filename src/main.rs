#![deny(clippy::all, clippy::pedantic, rust_2018_idioms)]

use crate::server::{Server, SignalName};
use clap::{AppSettings, Clap};

use protocol::Protocol;
use server::Shutdown;
use std::process;
use tokio::{
    net::{TcpListener, UdpSocket},
    signal::{self, unix::SignalKind},
    sync::broadcast::{self, Sender},
};
use tracing::subscriber::set_global_default;
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

mod protocol;
mod server;

#[tokio::main]
#[allow(clippy::semicolon_if_nothing_returned)]
async fn main() {
    let opts = {
        let mut o = Opts::parse();
        o.proto.sort();
        o.proto.dedup();
        o
    };

    init_logging(&opts.log_level);

    tracing::debug!(?opts);

    let (shutdown_tx, _) = broadcast::channel(1);

    tracing::debug!(
        host = %opts.address,
        port = %opts.port,
        "server listen info"
    );

    let tasks = opts
        .proto
        .iter()
        .map(|proto| match proto {
            Protocol::Tcp => tokio::spawn(start_tcp_listener(
                opts.address.clone(),
                opts.port,
                shutdown_tx.subscribe(),
            )),
            Protocol::Udp => tokio::spawn(start_udp_listener(
                opts.address.clone(),
                opts.port,
                shutdown_tx.subscribe(),
            )),
        })
        .collect::<Vec<_>>();

    signal_handler(shutdown_tx.clone()).await;

    futures_util::future::join_all(tasks).await;

    tracing::info!("shutting down");
}

async fn signal_handler(shutdown: Sender<SignalName>) {
    let sigint = signal::ctrl_c();
    let mut sigterm = signal::unix::signal(SignalKind::terminate()).unwrap();
    let sig = tokio::select! {
        _ = sigint => SignalName::SigInt,
        _ = sigterm.recv() => SignalName::SigTerm
    };
    tracing::info!(?sig, "received signal");
    tracing::trace!(?sig, "sending signal");
    match shutdown.send(sig) {
        Ok(_) => tracing::trace!(?sig, "signal sent"),
        Err(err) => tracing::error!(?err, "error signaling workers"),
    }
}

async fn start_tcp_listener(addr: String, port: u16, shutdown: Shutdown) {
    let tcp_server = match TcpListener::bind((addr.as_str(), port)).await {
        Ok(s) => s,
        Err(err) => {
            tracing::error!(?err, "error starting TCP server");
            process::exit(2);
        }
    };

    tracing::info!(
        host = %addr,
        %port,
        "tcp server listening"
    );

    tcp_server.run(shutdown).await;
}

async fn start_udp_listener(addr: String, port: u16, shutdown: Shutdown) {
    let udp_server = match UdpSocket::bind((addr.as_str(), port)).await {
        Ok(s) => s,
        Err(err) => {
            tracing::error!(?err, "error starting UDP server");
            process::exit(2);
        }
    };

    tracing::info!(
        host = %addr,
        %port,
        "udp server listening"
    );

    udp_server.run(shutdown).await;
}

#[derive(Debug, Clap)]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    #[clap(short, long, env("DAYTIME_ADDR"), default_value = "0.0.0.0")]
    pub address: String,

    /// Comma-delimited list of protocols to listen on.
    #[clap(short('P'), long, env("DAYTIME_PROTO"), use_delimiter(true), default_values=&["tcp", "udp"])]
    pub proto: Vec<Protocol>,

    #[clap(short, long, env("DAYTIME_PORT"), default_value = "13")]
    pub port: u16,

    #[clap(short, long, env("DAYTIME_LOG_LEVEL"), default_value = "info")]
    pub log_level: String,
}

fn init_logging(default_level: &str) {
    LogTracer::init().expect("log tracer");
    let filter = EnvFilter::try_from_env("DAYTIME_LOG_LEVEL")
        .unwrap_or_else(|_| EnvFilter::new(default_level));
    let subscriber = tracing_subscriber::fmt::layer();
    let registry = Registry::default().with(filter).with(subscriber);
    set_global_default(registry).expect("set registry");
}
