use anyhow::Result;
use tower_http::services::ServeDir;

use crate::state::AppState;

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
    tracing_subscriber::fmt::init();

    let conn = db::connect().await?;
    db::ensure_schema(&conn).await?;

    let state = AppState {
        db: conn,
        client: state::build_client()?,
    };

    let app = views::router()
        .with_state(state.clone())
        .nest_service("/static", ServeDir::new("static"));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    let server = axum::serve(listener, app);

    tokio::select! {
        r = server => {
            println!("Server stopped");
            r.map_err(Into::into)
        }
        r = poller::poll_loop(state) => {
            println!("Poll loop stopped");
            r
        }
    }
}

