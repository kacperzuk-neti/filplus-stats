use axum::{routing::get, Router};
use color_eyre::eyre::WrapErr;
pub use color_eyre::Result;
use http::Method;
use sqlx::postgres::{PgPool, PgPoolOptions};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{filter::EnvFilter, FmtSubscriber};

mod allocators;
mod error;
mod providers;
mod types;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug,sqlx=info"));

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(env_filter)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let db_url =
        std::env::var("DATABASE_URL").context("Error reading env variable DATABASE_URL")?;
    let pool: PgPool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;
    // build our application with a single route
    let app = Router::new()
        .route(
            "/stats/providers/retrievability",
            get(providers::providers_retrievability),
        )
        .route(
            "/stats/providers/clients",
            get(providers::providers_clients),
        )
        .route(
            "/stats/providers/biggest_client_distribution",
            get(providers::providers_biggest_client_distribution),
        )
        .route(
            "/stats/allocators/retrievability",
            get(allocators::allocators_retrievability),
        )
        .route(
            "/stats/allocators/biggest_client_distribution",
            get(allocators::allocators_biggest_client_distribution),
        )
        .route(
            "/stats/allocators/sps_compliance",
            get(allocators::allocators_sps_compliance),
        )
        .layer(
            CorsLayer::new()
                .allow_methods([Method::GET])
                .allow_headers(Any)
                .allow_origin(Any),
        )
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
