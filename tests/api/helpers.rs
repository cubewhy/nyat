use anyhow::Context;
use sqlx::Executor;
use std::sync::LazyLock;

use nyat::{
    auth::{generate_token, hash_password},
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
    pub db: PgPool,
    pub test_user: TestUser,

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

    pub async fn login(&self, username: &str, password: &str) -> reqwest::Response {
        self.http_client
            .post(format!("{}/user/login", self.address))
            .json(&json!({
                "username": username,
                "password": password
            }))
            .send()
            .await
            .unwrap()
    }

    pub async fn create_test_user(&self) -> TestUser {
        // request the register api
        let username = Uuid::new_v4().to_string().replace("-", "_");
        let password = Uuid::new_v4().to_string();
        let res = self.register(&username, &password).await;

        // make sure the request success
        assert_eq!(res.status().as_u16(), 200);

        // extract the token field from the response
        let json = res.json::<serde_json::Value>().await.unwrap();
        let json = json.as_object().unwrap();

        let token = json
            .get("token")
            .context("Field \"token\" not exist on register response")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();

        // query the user id from the database
        let id = sqlx::query!("SELECT id FROM users WHERE username = $1", username)
            .fetch_one(&self.db)
            .await
            .unwrap()
            .id;

        TestUser {
            id,
            username,
            password,
            token,
        }
    }

    pub async fn create_pm(&self, token: &str, peer_username: &str) -> reqwest::Response {
        self.http_client
            .post(format!("{}/chat/pm", self.address))
            .json(&json!({
                "peer_username": peer_username
            }))
            .bearer_auth(token)
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
    let db = configure_database(&config.database).await;

    // insert the test admin user into the database
    let test_user = insert_test_user(&db, config.security.token_secret.as_bytes()).await;

    // build the application
    let app = Application::build(config).await.unwrap();
    let port = app.port();

    // run the application at background
    tokio::spawn(app.run_until_stopped());

    TestApp {
        port,
        address: format!("http://localhost:{port}"),
        db,
        test_user,
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

pub struct TestUser {
    pub id: i64,
    pub username: String,
    pub password: String,
    pub token: String,
}

async fn insert_test_user(pool: &PgPool, token_secret: &[u8]) -> TestUser {
    let username = "test_user";
    let password = "testtest";
    let hashed_password = hash_password(password).unwrap();
    let test_user_id = sqlx::query!(
        "INSERT INTO users (username, password) VALUES ($1, $2) RETURNING (id);",
        username,
        hashed_password
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .id;

    // generate the token for test user
    let token = generate_token(test_user_id, 3600, token_secret).unwrap();

    TestUser {
        id: test_user_id,
        username: username.to_string(),
        password: password.to_string(),
        token,
    }
}
