use anyhow::Result;
use tracing_subscriber::EnvFilter;

use crate::state::AppState;

mod assets;
mod components;
mod db;
mod entities;
mod error;
mod models;
mod notify;
mod poller;
mod state;
mod utils;
mod views;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let conn = db::connect().await?;
    tracing::info!("database connected");
    db::ensure_schema(&conn).await?;
    tracing::info!("database schema ready");

    let state = AppState {
        db: conn,
        client: state::build_client()?,
    };

    let app = views::router().with_state(state.clone());
    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("listening on {addr}");
    let server = axum::serve(listener, app).with_graceful_shutdown(shutdown_signal());

    tokio::select! {
        r = server => {
            tracing::info!("server stopped");
            r.map_err(Into::into)
        }
        r = poller::poll_loop(state) => {
            tracing::info!("poll loop stopped");
            r
        }
    }
}

/// Docker runs the container's entrypoint as PID 1, which has no default signal
/// handlers, so SIGTERM must be handled explicitly to shut down gracefully.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install SIGINT handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("received SIGINT, shutting down"),
        _ = terminate => tracing::info!("received SIGTERM, shutting down"),
    }
}

