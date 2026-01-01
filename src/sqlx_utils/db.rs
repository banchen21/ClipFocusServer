use sqlx::{
    Row, query,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool},
};
use uuid::Uuid;
use yansi::Paint;

use crate::user_api::{LoginUser, RegisterUser, User, UserInfo};

/// 初始化 SQLite 连接池
///
/// 该函数将创建一个 SQLite 连接池，连接到指定文件名的数据库文件中
///
/// - 文件名：`data.db`，可以根据需要进行修改
/// - 允许创建文件：`create_if_missing` 选项设置为 `true`，表示如果文件不存在，将自动创建
/// - 日志模式：`journal_mode` 选项设置为 `SqliteJournalMode::Wal`，表示使用WAL日志模式，可以提高性能
/// - 锁超时设置：`busy_timeout` 选项设置为 `std::time::Duration::from_secs(5)`，表示如果在5秒内没有可用的连接，将返回错误
pub async fn init_pool() -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename("data.db") // 显式指定文件名
        .create_if_missing(true) // ✅ 关键修复：允许创建文件
        .journal_mode(SqliteJournalMode::Wal) // 推荐WAL模式提升性能
        .busy_timeout(std::time::Duration::from_secs(5)); // 锁超时设置
    sqlx::SqlitePool::connect_with(options).await
}

/// 用户表结构定义
const CREATE_USERS_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS users (
    user_id TEXT PRIMARY KEY NOT NULL,
    username TEXT NOT NULL,
    email TEXT UNIQUE NOT NULL,
    password TEXT NOT NULL,
    head_uri TEXT 
);

CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
"#;

// 初始化数据库
pub async fn crate_db(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(CREATE_USERS_TABLE_SQL).execute(pool).await?;
    Ok(())
}

// 插入后返回用户 ID
pub async fn insert_user(
    register_user: &RegisterUser,
    pool: &SqlitePool,
) -> Result<String, sqlx::Error> {
    let user_id = Uuid::new_v4().to_string();
    query(
        r#"
        INSERT INTO users (user_id, username, email, password)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(&user_id)
    .bind(register_user.username.clone())
    .bind(register_user.email.clone())
    .bind(register_user.password.clone())
    .execute(pool)
    .await?;
    Ok(user_id)
}

// 根据用户名或者 email 查询用户信息
pub async fn get_user_by_username_or_email(
    username_or_email: &str,
    pool: &SqlitePool,
) -> Result<User, sqlx::Error> {
    let row = query(
        r#"
        SELECT user_id, username, email, password, head_uri
        FROM users
        WHERE username = $1 OR email = $2
        "#,
    )
    .bind(username_or_email)
    .bind(username_or_email)
    .fetch_optional(pool)
    .await?;

    Ok(match row {
        Some(row) => User {
            user_id: row.try_get("user_id")?,
            username_or_email: row.try_get("username")?,
            password: row.try_get("password")?,
        },
        None => return Err(sqlx::Error::RowNotFound),
    })
}

// 修改用户名
pub async fn update_username(
    user_id: &str,
    username: &str,
    pool: &SqlitePool,
) -> Result<(), sqlx::Error> {
    query(
        r#"
        UPDATE users
        SET username = $2
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .bind(username)
    .execute(pool)
    .await?;
    Ok(())
}

// 修改头像
pub async fn update_head_uri(
    user_id: &str,
    head_uri: &str,
    pool: &SqlitePool,
) -> Result<(), sqlx::Error> {
    query(
        r#"
        UPDATE users
        SET head_uri = $2
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .bind(head_uri)
    .execute(pool)
    .await?;
    Ok(())
}

// 修改密码
pub async fn update_password(
    user_id: &str,
    new_password: &str,
    pool: &SqlitePool,
) -> Result<(), sqlx::Error> {
    query(
        r#"
        UPDATE users
        SET password = $2
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .bind(new_password)
    .execute(pool)
    .await?;
    Ok(())
}

// 获取用户信息
pub async fn get_user_by_id(user_id: &str, pool: &SqlitePool) -> Result<UserInfo, sqlx::Error> {
    let row = query(
        r#"
        SELECT user_id, username, email, password, head_uri
        FROM users
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(UserInfo {
        username: row.try_get("username")?,
        email: row.try_get("email")?,
        head_uri: row.try_get("head_uri")?,
    })
}