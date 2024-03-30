use poem::{listener::TcpListener, Route, Server};
use poem_openapi::OpenApiService;

mod api;

#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    tracing::info!("Starting OpenApi service and server...");

    let api_service =
        OpenApiService::new(api::Api, "Hello World", "1.0").server("http://localhost:3000");

    let app = Route::new().nest("/", api_service);

    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await?;

    tracing::info!("Starting OpenApi service and server...");

    Ok(())
}
