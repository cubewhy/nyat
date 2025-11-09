use config::Config;
use url::Url;

#[derive(serde::Deserialize)]
pub struct Settings {
    pub application: ApplicationConfig,
    pub database: DatabaseConfig,
    pub security: SecurityConfig,
}

#[derive(serde::Deserialize)]
pub struct ApplicationConfig {
    pub host: String,
    pub port: u16,
}

#[derive(serde::Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: Url,
}

#[derive(serde::Deserialize)]
pub struct SecurityConfig {
    pub token_expire_interval: usize,
    pub token_secret: String,
}

impl DatabaseConfig {
    pub fn set_db_name(&mut self, db_name: &str) {
        self.url.set_path(db_name);
    }

    pub fn db_url(&self) -> String {
        self.url.to_string()
    }

    pub fn db_name(&self) -> &str {
        self.url.path().trim_start_matches("/")
    }
}

pub fn load_config() -> anyhow::Result<Settings> {
    let base_path = std::env::current_dir().expect("Failed to determinate current dir");
    let config_dir = base_path.join("config");
    // TODO: Load environment specific configuration
    let settings = Config::builder()
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .add_source(config::File::from(config_dir.join("base.yaml")))
        .build()?;

    Ok(settings.try_deserialize()?)
}
