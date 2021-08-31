#![deny(clippy::all, clippy::pedantic, rust_2018_idioms)]

use std::process;

use clap::Clap;
use protocol::Protocol;
use tokio::{
    net::{TcpListener, UdpSocket},
    signal::{self, unix::SignalKind},
    sync::broadcast::{self, Sender},
};
use tracing::subscriber::set_global_default;
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

use crate::server::{Server, SignalName};

mod protocol;
mod server;

#[tokio::main]
#[allow(clippy::semicolon_if_nothing_returned)]
async fn main() {
    let opts = Opts::parse();

    init_logging(&opts.log_level);

    tracing::debug!(?opts);

    let (shutdown_tx, _) = broadcast::channel(1);

    tracing::debug!(
        host = %opts.address,
        port = %opts.port,
        "server listen info"
    );

    let tcp_server = match TcpListener::bind((opts.address.as_str(), opts.port)).await {
        Ok(s) => s,
        Err(err) => {
            tracing::error!(?err, "error starting TCP server");
            process::exit(2);
        }
    };

    tracing::info!(
        host = %opts.address,
        port = %opts.port,
        "tcp server listening"
    );

    let tcp_server = tcp_server.run(shutdown_tx.subscribe());

    let udp_server = match UdpSocket::bind((opts.address.as_str(), opts.port)).await {
        Ok(s) => s,
        Err(err) => {
            tracing::error!(?err, "error starting UDP server");
            process::exit(2);
        }
    };

    tracing::info!(
        host = %opts.address,
        port = %opts.port,
        "udp server listening"
    );

    let udp_server = udp_server.run(shutdown_tx.subscribe());

    tokio::spawn(signal_handler(shutdown_tx));

    futures_util::future::join(udp_server, tcp_server).await;
}

async fn signal_handler(shutdown: Sender<SignalName>) {
    let sigint = signal::ctrl_c();
    let mut sigterm = signal::unix::signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = sigint => shutdown.send(SignalName::SigInt).unwrap(),
        _ = sigterm.recv() => shutdown.send(SignalName::SigTerm).unwrap()
    };
    // let _send = shutdown.send(SignalKind::interrupt());
}

#[derive(Debug, Clap)]
struct Opts {
    #[clap(short, long, env("DAYTIME_ADDR"), default_value = "0.0.0.0")]
    pub address: String,

    #[clap(short('P'), long, env("DAYTIME_PROTO"))]
    pub proto: Option<Protocol>,

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
