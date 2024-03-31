use poem::{listener::TcpListener, Route, Server};
use poem_openapi::OpenApiService;

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

    // Start server
    let api_service =
        OpenApiService::new(api::Api, "Hello World", "1.0").server("http://localhost:3000");
    let ui = api_service.openapi_explorer();
    let app = Route::new().nest("/", api_service).nest("/docs", ui);

    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await?;

    Ok(())
}
