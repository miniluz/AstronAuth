use axum::{routing, Router};

mod api;
mod query;

#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    // Initialize color_eyre
    color_eyre::install()?;

    // Initialize tracing subscriber
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let app = Router::new().route("/", routing::get(api::index));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
