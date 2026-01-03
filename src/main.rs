mod sqlx_utils;
mod user_api;
mod spatial_api;
mod utils;

use actix::Actor;
use actix_web::{App, HttpServer, error as actix_error, web};
use actix_cors::Cors; // 引入 CORS
use dotenvy::dotenv;
use log::info;
use std::error::Error;

use crate::spatial_api::models::{AppState, RoomManager};
use crate::sqlx_utils::db::init_pool;
use crate::user_api::user_api;
use crate::spatial_api::ws_api;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let http_port = 3000;

    // 初始化数据库连接池
    let pool = init_pool().await?;
    sqlx_utils::db::crate_db(&pool)
        .await
        .map_err(actix_error::ErrorInternalServerError)
        .err();

    // 初始化房间管理器 Actor
    let room_manager = RoomManager::new().start();

    // 创建共享状态
    let app_state = AppState {
        room_manager: room_manager.clone(),
    };

    info!("Starting Actix-Web server on http://127.0.0.1:{}", http_port);

    HttpServer::new(move || {
        // 配置 CORS
        let cors = Cors::default()
            .allow_any_origin() // 允许所有来源访问，可根据需求改为 .allowed_origin("http://tauri.localhost")
            .allow_any_method() // 允许 GET, POST 等请求方法
            .allow_any_header() // 允许所有请求头
            .supports_credentials(); // 如果需要发送 Cookie 或授权头

        App::new()
            .wrap(cors) // 使用 CORS 中间件
            .app_data(web::Data::new(app_state.clone()))
            .app_data(web::Data::new(pool.clone()))
            .service(web::scope("/api/v1")
                .service(user_api())
                .service(ws_api())
            )
    })
    .bind(("0.0.0.0", http_port))?
    .run()
    .await?;

    Ok(())
}
