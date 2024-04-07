use axum::{routing, Router};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod auth;

#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    #[derive(OpenApi)]
    #[openapi(
        paths(
            auth::authorization
        ),
        components(
        ),
        tags(
            (name = "auth", description = "OAuth 2.0 authentication API")
        )
    )]
    struct ApiDoc;

    // Initialize color_eyre
    color_eyre::install()?;

    // Initialize tracing subscriber
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let app = Router::new()
        .route("/authorization", routing::get(auth::authorization))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
