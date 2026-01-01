use actix_web::web::Json;
use serde::de::{self};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// 响应数据类型枚举
#[derive(Debug)]
pub enum ResponseData {
    /// 空数据
    Null,
    /// 字符串数据
    Text(String),
    /// 二进制数据（Base64编码）
    Binary(Vec<u8>),
    /// JSON对象
    Json(serde_json::Value),
    /// 布尔值
    Boolean(bool),
    /// 数字
    Number(i64),
    /// 浮点数
    Float(f64),
    /// 数组
    Array(Vec<ResponseData>),
}

/// 手动实现 Serialize
impl Serialize for ResponseData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ResponseData::Null => serializer.serialize_none(),
            ResponseData::Text(s) => serializer.serialize_str(s),
            ResponseData::Binary(data) => {
                // 将二进制数据转换为Base64字符串
                let base64_string = base64::encode(data);
                serializer.serialize_str(&base64_string)
            }
            ResponseData::Json(v) => v.serialize(serializer),
            ResponseData::Boolean(b) => serializer.serialize_bool(*b),
            ResponseData::Number(n) => serializer.serialize_i64(*n),
            ResponseData::Float(f) => serializer.serialize_f64(*f),
            ResponseData::Array(arr) => arr.serialize(serializer),
        }
    }
}

/// 手动实现 Deserialize
impl<'de> Deserialize<'de> for ResponseData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // 使用serde_json的Value作为中间表示
        let value = serde_json::Value::deserialize(deserializer)?;

        match value {
            serde_json::Value::Null => Ok(ResponseData::Null),
            serde_json::Value::Bool(b) => Ok(ResponseData::Boolean(b)),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(ResponseData::Number(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(ResponseData::Float(f))
                } else {
                    Err(de::Error::custom("无法解析的数字类型"))
                }
            }
            serde_json::Value::String(s) => {
                // 尝试判断是否是Base64编码的二进制数据
                // 这里简单判断，实际中可能需要更复杂的逻辑
                if s.len() % 4 == 0
                    && s.chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
                {
                    match base64::decode(&s) {
                        Ok(data) => Ok(ResponseData::Binary(data)),
                        Err(_) => Ok(ResponseData::Text(s)),
                    }
                } else {
                    Ok(ResponseData::Text(s))
                }
            }
            serde_json::Value::Array(arr) => {
                let mut result = Vec::new();
                for item in arr {
                    result.push(ResponseData::deserialize(item).map_err(de::Error::custom)?);
                }
                Ok(ResponseData::Array(result))
            }
            serde_json::Value::Object(obj) => {
                Ok(ResponseData::Json(serde_json::Value::Object(obj)))
            }
        }
    }
}

/// 主要响应结构
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    pub message: String,
    pub data: ResponseData,
    pub timestamp: i64,
}

impl ApiResponse {
    pub fn new(message: &str, data: ResponseData) -> Json<ApiResponse> {
        let api_response= ApiResponse {
            message: message.to_string(),
            data,
            timestamp: chrono::Utc::now().timestamp(),
        };
        Json(api_response)
    }
}
