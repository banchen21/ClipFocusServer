mod sqlx_utils;

mod user_api;
use actix::Actor;
use actix_web::{App, HttpServer, error as actix_error, web};
use dotenvy::dotenv;
use log::info;
use std::error::Error;
mod utils;

use crate::spatial_api::models::{AppState, RoomManager};
use crate::sqlx_utils::db::init_pool;
use crate::user_api::user_api;
mod spatial_api;
use crate::spatial_api::ws_api;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let http_port = 3000;

    let pool = init_pool().await?;
    sqlx_utils::db::crate_db(&pool)
        .await
        .map_err(actix_error::ErrorInternalServerError)
        .err();
    // 初始化数据库

    // 创建房间管理器Actor
    let room_manager = RoomManager::new().start();

    // 创建共享状态
    let app_state = AppState {
        room_manager: room_manager.clone(),
    };

    info!(
        "Starting Actix-Web server on http://127.0.0.1:{:?}",
        http_port
    );
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .app_data(web::Data::new(pool.clone()))
            .service(web::scope("/api/v1").service(user_api()).service(ws_api()))
    })
    .bind(("0.0.0.0", http_port))?
    .run()
    .await?;

    Ok(())
}
