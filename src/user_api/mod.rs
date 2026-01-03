use actix_web::{Responder, get, post, put, web};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::SqlitePool;

use crate::{
    sqlx_utils::{
        db,
        models::{ApiResponse, ResponseData},
    },
    user_api::auth::{BearerToken, generate_access_token},
    utils::save_payload_with_dirs,
};

pub(crate) mod auth;

pub fn user_api() -> actix_web::Scope {
    return web::scope("/user")
        .service(register)
        .service(login)
        .service(refresh_token)
        .service(change_nickname)
        .service(change_head)
        .service(change_password)
        .service(get_user_info);
}
 
#[derive(Debug, Deserialize)]
pub struct User {
    pub user_id: String,
    pub username_or_email: String,
    pub password: String,
}
// 用户注册
#[derive(Deserialize)]
pub struct RegisterUser {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[post("/register")]
async fn register(
    pool: web::Data<SqlitePool>,
    register_user: web::Json<RegisterUser>,
) -> impl Responder {
    // 插入后返回用户 ID
    match db::insert_user(&register_user.0, &pool).await {
        Ok(user_id) => match generate_access_token(&user_id, &register_user.username) {
            Ok(token) => ApiResponse::new("注册成功", ResponseData::Text(token)),
            Err(_err) => ApiResponse::new("注册失败", ResponseData::Null),
        },
        Err(_) => ApiResponse::new("注册失败", ResponseData::Null),
    }
}

// 刷新 Token
#[post("/refresh_token")]
async fn refresh_token(bearer_token: BearerToken) -> impl Responder {
    info!("刷新令牌请求");

    // 生成新的访问令牌
    let access_token = match generate_access_token(&bearer_token.user_id, &bearer_token.username) {
        Ok(token) => token,
        Err(e) => {
            warn!("生成新访问令牌失败: {}", e);
            return ApiResponse::new(&e, ResponseData::Null);
        }
    };

    ApiResponse::new("令牌刷新成功", ResponseData::Text(access_token))
}

// 用户登录
#[derive(Deserialize)]
pub struct LoginUser {
    pub username_or_email: String,
    pub password: String,
}

#[post("/login")]
async fn login(pool: web::Data<SqlitePool>, login_user: web::Json<LoginUser>) -> impl Responder {
    info!("用户请求登录");
    match db::get_user_by_username_or_email(&login_user.username_or_email, &pool).await {
        Ok(user) => {
            debug!("用户信息: {:#?}", user);
            if user.password == login_user.password {
                match generate_access_token(&user.user_id, &user.username_or_email) {
                    Ok(token) => ApiResponse::new("登录成功", ResponseData::Text(token)),
                    Err(_err) => ApiResponse::new("登录失败", ResponseData::Null),
                }
            } else {
                ApiResponse::new("登录失败", ResponseData::Null)
            }
        }
        Err(_) => ApiResponse::new("登录失败", ResponseData::Null),
    }
}

// 修改昵称
#[derive(Deserialize)]
pub struct ChangeNickName {
    new_nickname: String,
}

#[put("/change_nickname")]
async fn change_nickname(
    pool: web::Data<SqlitePool>,
    bearer_token: BearerToken,
    register_user: web::Query<ChangeNickName>,
) -> impl Responder {
    info!("新昵称:{}", register_user.new_nickname);
    match db::update_username(&bearer_token.user_id, &bearer_token.username, &pool).await {
        Ok(_) => ApiResponse::new(
            "昵称修改成功",
            ResponseData::Text(
                match generate_access_token(&bearer_token.user_id, &bearer_token.username) {
                    Ok(token) => token,
                    Err(_err) => _err,
                },
            ),
        ),
        Err(_) => ApiResponse::new("昵称修改失败", ResponseData::Null),
    }
}

#[put("/change_head")]
async fn change_head(
    pool: web::Data<SqlitePool>,
    bearer_token: BearerToken,
    payload: web::Payload,
) -> impl Responder {
    info!("修改头像");
    let uuid = uuid::Uuid::new_v4();
    // 将_data保存到本地
    let file_path = format!("./static/heads/{}", uuid);
    match save_payload_with_dirs(payload, &file_path).await {
        Ok(_) => match db::update_head_uri(&bearer_token.user_id, &uuid.to_string(), &pool).await {
            Ok(_) => ApiResponse::new(
                "头像修改成功",
                ResponseData::Text(
                    match generate_access_token(&bearer_token.user_id, &bearer_token.username) {
                        Ok(token) => token,
                        Err(_err) => _err,
                    },
                ),
            ),
            Err(_) => ApiResponse::new("头像修改失败", ResponseData::Null),
        },
        Err(_) => todo!(),
    }
}

// 修改密码
#[derive(Deserialize)]
pub struct ChangePassword {
    new_password: String,
}

#[put("/change_password")]
async fn change_password(
    pool: web::Data<SqlitePool>,
    bearer_token: BearerToken,
    change_password: web::Query<ChangePassword>,
) -> impl Responder {
    info!("新密码:{}", change_password.new_password);
    match db::update_password(&bearer_token.user_id, &change_password.new_password, &pool).await {
        Ok(_) => ApiResponse::new(
            "密码修改成功",
            ResponseData::Text(
                match generate_access_token(&bearer_token.user_id, &bearer_token.username) {
                    Ok(token) => token,
                    Err(_err) => _err,
                },
            ),
        ),
        Err(_) => ApiResponse::new("密码修改失败", ResponseData::Null),
    }
}

#[derive(Serialize, Deserialize)]
pub struct UserInfo {
    pub username: String,
    pub email: String,
    pub head_uri: String,
}

// 获取用户信息
#[get("/get_user_info")]
async fn get_user_info(pool: web::Data<SqlitePool>, bearer_token: BearerToken) -> impl Responder {
    info!("获取用户信息请求");
    match db::get_user_by_id(&bearer_token.user_id, &pool).await {
        Ok(user) => ApiResponse::new("获取用户信息成功", ResponseData::Json(json!(user))),
        Err(_) => ApiResponse::new("获取用户信息失败", ResponseData::Null),
    }
}
