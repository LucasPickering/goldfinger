mod api;
mod state;
mod reducer;

use log::LevelFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Trace)
        .init();

    api::start().await?;

    Ok(())
}
