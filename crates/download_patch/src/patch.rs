use std::io::SeekFrom;
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

#[allow(dead_code)]
pub async fn patch(old_file_path: impl AsRef<Path>, patch_file_path: impl AsRef<Path>, new_file_path: impl AsRef<Path>) -> Result<(), tokio::io::Error> {
    let mut new_file = File::create(new_file_path).await?;
    let mut old_file = File::open(old_file_path).await?;
    let mut patch_file = File::open(patch_file_path).await?;

    let max_size = patch_file.read_u32_le().await? as usize;
    let mut buffer = vec![0_u8; max_size];
    let mut u8_bytes = [0u8; 1];
    let mut u64_bytes = [0u8; 8];
    let mut u32_bytes = [0u8; 4];
    loop {
        let bytes_read = patch_file.read(&mut u8_bytes).await?;
        if bytes_read == 0 {
            break;
        }
        let op = u8::from_ne_bytes(u8_bytes);
        match op {
            0 => {
                patch_file.read_exact(&mut u64_bytes).await?;
                let offset = u64::from_le_bytes(u64_bytes);
                patch_file.read_exact(&mut u32_bytes).await?;
                let length = u32::from_le_bytes(u32_bytes) as usize;
                old_file.seek(SeekFrom::Start(offset)).await?;
                old_file.read_exact(&mut buffer[0..length]).await?;
                new_file.write_all(&mut buffer[0..length]).await?;
            }
            _ => {
                patch_file.read_exact(&mut u32_bytes).await?;
                let length = u32::from_le_bytes(u32_bytes) as usize;
                patch_file.read_exact(&mut buffer[0..length]).await?;
                new_file.write_all(&mut buffer[0..length]).await?;
            }
        }
    }
    Ok(())
}