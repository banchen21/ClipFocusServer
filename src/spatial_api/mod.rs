pub mod models;
use actix::{Actor, StreamHandler};
use actix_web::{App, Error, HttpRequest, HttpResponse, HttpServer, get, web};
use actix_web_actors::ws;

use crate::{spatial_api::models::{AppState, MyWs}, user_api::auth::BearerToken};

pub fn ws_api() -> actix_web::Scope {
    return web::scope("/spatial").service(index);
}

// WebSocket端点
#[get("/ws")]
async fn index(
    bearer_token: BearerToken,
    req: HttpRequest,
    stream: web::Payload,
    data: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let user_id = bearer_token.user_id;
    
    println!("WebSocket connection requested for user: {}", user_id);
    
    let resp = ws::start(
        MyWs::new(user_id, data.room_manager.clone()),
        &req,
        stream,
    );
    
    println!("WebSocket response: {:?}", resp);
    resp
}