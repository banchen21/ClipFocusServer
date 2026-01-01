use actix_web::{web, Error};
use futures::StreamExt;
use tokio::{fs, io::AsyncWriteExt};

pub async fn save_payload_with_dirs(
    mut payload: web::Payload,
    file_path: &str,
) -> Result<(), Error> {
    // 自动创建目录
    if let Some(parent) = std::path::Path::new(file_path).parent() {
        fs::create_dir_all(parent).await?;
    }
    
    // 创建文件并写入数据
    let mut file = fs::File::create(file_path).await?;
    
    while let Some(chunk) = payload.next().await {
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk?).await?;
    }
    
    Ok(())
}