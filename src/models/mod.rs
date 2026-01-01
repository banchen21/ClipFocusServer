// clipfocus-core/src/models/mod.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 剪贴板内容类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "clip_type", rename_all = "snake_case")]
pub enum ClipType {
    Text,           // 纯文本
    Html,           // HTML内容
    Url,            // 链接
    FilePath,       // 文件路径
    Image,          // 图片数据
    Rtf,            // RTF格式
    Unknown,        // 未知类型
}

/// 剪贴板项目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipItem {
    pub id: Uuid,
    pub device_id: Uuid,
    pub content_type: ClipType,
    
    /// 原始内容（如果是文本类型，直接存储；如果是图片，存储base64或路径）
    pub content: String,
    
    /// 内容的简单预览（截取前200个字符或生成缩略图描述）
    pub preview: String,
    
    /// 内容大小（字节数）
    pub size: i64,
    
    /// 源应用（如果可获取）
    pub source_app: Option<String>,
    
    /// 创建时间
    pub created_at: DateTime<Utc>,
    
    /// 最后访问时间
    pub accessed_at: DateTime<Utc>,
    
    /// 同步状态
    pub sync_status: SyncStatus,
    
    /// 加密状态
    pub encrypted: bool,
    
    /// 标签/分类
    pub tags: Vec<String>,
}

/// 同步状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "sync_status", rename_all = "snake_case")]
pub enum SyncStatus {
    Local,          // 仅本地
    Syncing,        // 同步中
    Synced,         // 已同步
    Conflict,       // 冲突
}

/// 剪贴板项目创建请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateClipRequest {
    pub device_id: Uuid,
    pub content_type: ClipType,
    pub content: String,
    pub preview: Option<String>,
    pub source_app: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// 剪贴板项目更新请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateClipRequest {
    pub accessed: bool,
    pub tags: Option<Vec<String>>,
}

/// 剪贴板项目查询过滤器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipFilter {
    pub clip_type: Option<ClipType>,
    pub device_id: Option<Uuid>,
    pub tags: Option<Vec<String>>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub search_text: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}




