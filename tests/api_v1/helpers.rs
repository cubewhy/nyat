use sqlx::Executor;
use std::sync::LazyLock;

use nyat::{
    configuration::{DatabaseConfig, load_config},
    startup::Application,
    telemetry::{get_subscriber, init_subscriber},
};
use serde_json::json;
use sqlx::{Connection, PgConnection, PgPool};
use uuid::Uuid;

static TRACING: LazyLock<()> = LazyLock::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

pub struct TestApp {
    pub address: String,
    pub port: u16,

    http_client: reqwest::Client,
}

impl TestApp {
    pub async fn register(&self, username: &str, password: &str) -> reqwest::Response {
        self.http_client
            .post(format!("{}/user/register", self.address))
            .json(&json!({
                "username": username,
                "password": password
            }))
            .send()
            .await
            .unwrap()
    }
}

pub async fn spawn_app() -> TestApp {
    // Init logger
    LazyLock::force(&TRACING);

    // load configuration
    let config = {
        let mut c = load_config().unwrap();
        // use random port
        c.application.port = 0;
        // make sure we do not expose the test instance to the public web
        c.application.host = "127.0.0.1".to_string();

        // use a random database name
        c.database.set_db_name(&Uuid::new_v4().to_string());

        c
    };

    // Create and migrate the database
    configure_database(&config.database).await;

    // build the application
    let app = Application::build(config).await.unwrap();
    let port = app.port();

    // run the application at background
    tokio::spawn(app.run_until_stopped());

    TestApp {
        port,
        address: format!("http://localhost:{port}"),

        http_client: reqwest::Client::new(),
    }
}

async fn configure_database(config: &DatabaseConfig) -> PgPool {
    // Create database
    let maintenance_settings = {
        let mut c = config.to_owned();
        c.set_db_name("postgres");

        c
    };

    let mut connection = PgConnection::connect(&maintenance_settings.db_url())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.db_name()).as_str())
        .await
        .expect("Failed to create database.");

    // Migrate database
    let connection_pool = PgPool::connect(&config.db_url())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
    connection_pool
}
