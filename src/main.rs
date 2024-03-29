use tracing::{info, warn};

fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("Just before hello world!");

    println!("Hello, world!");

    warn!("Just after hello world!");

    Ok(())
}
