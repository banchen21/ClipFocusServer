use actix_web::dev::Payload;
use actix_web::http::header;
use actix_web::{Error, FromRequest, HttpRequest};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::env;
use std::future::{Ready, ready};
use std::time::SystemTime;

use crate::sqlx_utils::models::{ApiResponse, ResponseData};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: String,
    pub username: String,
    pub exp: usize, // 过期时间戳
    pub iat: usize, // 签发时间戳
}

#[derive(Debug, Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

// 获取环境变量
fn get_secret(secret_name: &str) -> String {
    env::var(secret_name).unwrap_or_else(|_| {
        warn!(
            "{} not set, using default secret (insecure for production!)",
            secret_name
        );
        format!(
            "default-{}-secret-change-in-production",
            secret_name.to_lowercase()
        )
    })
}

// 生成令牌
pub fn generate_access_token(user_id: &str, username: &str) -> Result<String, String> {
    let secret = get_secret("JWT_SECRET");
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;

    let claims = Claims {
        user_id: user_id.to_string(),
        username: username.to_owned(),
        iat: now,
        exp: now + 15 * 60, // 15分钟
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| format!("Failed to generate token: {}", e))
}

// 验证令牌
pub fn validate_access_token(token: &str) -> Result<Claims, String> {
    let secret = get_secret("JWT_SECRET");

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| format!("Invalid token: {}", e))
}

pub struct BearerToken {
    pub user_id: String,
    pub username: String,
}

impl FromRequest for BearerToken {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let auth_header = req.headers().get(header::AUTHORIZATION);

        match auth_header {
            Some(header_value) => {
                if let Ok(auth_str) = header_value.to_str() {
                    if auth_str.starts_with("Bearer ") {
                        let token = auth_str[7..].trim().to_string();
                        // 验证刷新令牌
                        match validate_access_token(&token) {
                            Ok(claims) => ready(Ok(BearerToken {
                                user_id: claims.user_id,
                                username: claims.username,
                            })),
                            Err(_) => ready(Err(actix_web::error::ErrorBadRequest(
                                "无效的令牌格式",
                            ))),
                        }
                    } else {
                        ready(Err(actix_web::error::ErrorBadRequest(
                            "无效的令牌格式",
                        )))
                    }
                } else {
                    ready(Err(actix_web::error::ErrorBadRequest("无效的header")))
                }
            }
            None => ready(Err(actix_web::error::ErrorUnauthorized("缺少令牌"))),
        }
    }
}
