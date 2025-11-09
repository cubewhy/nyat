use std::net::TcpListener;

use actix_web::{App, HttpServer, dev::Server, web};
use bytes::Bytes;
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use crate::{
    configuration::Settings,
    routes::{login, register},
};

pub struct Application {
    server: Server,
    port: u16,
}

impl Application {
    pub async fn build(settings: Settings) -> anyhow::Result<Self> {
        let addr = format!(
            "{}:{}",
            settings.application.host, settings.application.port
        );
        let listener = TcpListener::bind(addr)?;
        let port = listener.local_addr()?.port();

        // build the server
        let server = run(
            listener,
            settings.database.db_url(),
            settings.security.token_expire_interval,
            Bytes::from(settings.security.token_secret),
        )
        .await?;

        Ok(Self { server, port })
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

pub struct TokenExpireInterval(pub usize);
pub struct TokenSecret(pub Bytes);

async fn run(
    lst: TcpListener,
    db_url: String,
    token_expire_interval: usize,
    token_secret: Bytes,
) -> anyhow::Result<Server> {
    // connect to postgres
    let pool = web::Data::new(PgPool::connect(&db_url).await?);

    let token_expire_interval = web::Data::new(TokenExpireInterval(token_expire_interval));
    let token_secret = web::Data::new(TokenSecret(token_secret));

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .app_data(pool.clone())
            .app_data(token_expire_interval.clone())
            .app_data(token_secret.clone())
            .route("/user/register", web::post().to(register))
            .route("/user/login", web::post().to(login))
    })
    .listen(lst)?
    .run();

    Ok(server)
}
