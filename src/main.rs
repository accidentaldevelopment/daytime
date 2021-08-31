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
use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt, EnvFilter, Registry};

use crate::server::Server;

mod protocol;
mod server;

#[tokio::main]
#[allow(clippy::semicolon_if_nothing_returned)]
async fn main() {
    init_logging("info");

    let opts = Opts::parse();

    tracing::debug!(?opts);

    let (shutdown_tx, _) = broadcast::channel(1);

    let tcp_server = match TcpListener::bind(format!("{}:{}", opts.address, opts.port)).await {
        Ok(s) => s,
        Err(err) => {
            tracing::error!(?err, "error starting TCP server");
            process::exit(2);
        }
    };

    let tcp_server = tcp_server.run(shutdown_tx.subscribe());

    let udp_server = match UdpSocket::bind(format!("{}:{}", opts.address, opts.port)).await {
        Ok(s) => s,
        Err(err) => {
            tracing::error!(?err, "error starting TCP server");
            process::exit(2);
        }
    };

    let udp_server = udp_server.run(shutdown_tx.subscribe());

    tokio::spawn(signal_handler(shutdown_tx));

    tokio::join!(tcp_server, udp_server);
}

async fn signal_handler(shutdown: Sender<SignalKind>) {
    let _sig = signal::ctrl_c().await;
    let _send = shutdown.send(SignalKind::interrupt());
}

#[derive(Debug, Clap)]
struct Opts {
    #[clap(short, long, env("DAYTIME_ADDR"), default_value = "0.0.0.0")]
    pub address: String,

    #[clap(short('P'), long, env("DAYTIME_PROTO"))]
    pub proto: Option<Protocol>,

    #[clap(short, long, env("DAYTIME_PORT"), default_value = "13")]
    pub port: u16,
}

fn init_logging(default_level: &str) {
    LogTracer::init().expect("log tracer");
    let filter = EnvFilter::try_from_env("DAYTIME_LOG_LEVEL")
        .unwrap_or_else(|_| EnvFilter::new(default_level));
    let subscriber = tracing_subscriber::fmt::layer().with_span_events(FmtSpan::FULL);
    let registry = Registry::default().with(filter).with(subscriber);
    set_global_default(registry).expect("set registry");
}
