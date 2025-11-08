use nyat::{
    configuration::load_config,
    startup::Application,
    telemetry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Init tracing logger
    let subscriber = get_subscriber("nya", "info", std::io::stdout);
    init_subscriber(subscriber);

    // load configuration
    let config = load_config()?;

    // create the application
    let app = Application::build(config).await?;
    app.run_until_stopped().await?;

    Ok(())
}
