use std::collections::HashMap;
use std::io::SeekFrom;
use std::path::Path;
#[cfg(feature = "patch")]
use fastcdc::v2020::{AsyncStreamCDC, ChunkData};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
#[cfg(feature = "patch")]
use tokio_stream::StreamExt;
use xxhash_rust::xxh64::xxh64;

#[allow(dead_code)]
#[cfg(feature = "patch")]
async fn get_file_chunks(file_path: impl AsRef<Path>, min_size: u32, avg_size: u32, max_size: u32) -> Result<HashMap<u64, HashMap<usize, ChunkData>>, tokio::io::Error> {
    let mut hashmap: HashMap<u64, HashMap<usize, ChunkData>> = HashMap::new();
    let file = File::open(file_path).await?;
    let mut chunks = AsyncStreamCDC::new(file, min_size, avg_size, max_size);
    let mut stream = Box::pin(chunks.as_stream());
    while let Some(result) = stream.next().await {
        let chunk = result.expect("failed to read chunk");
        let hash = xxh64(&chunk.data, 0);
        match hashmap.entry(hash) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                if !entry.get().contains_key(&chunk.length) {
                    entry.get_mut().insert(chunk.length, chunk);
                }
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                let mut map = HashMap::new();
                map.insert(chunk.length, chunk);
                entry.insert(map);
            }
        }
    }
    Ok(hashmap)
}

#[allow(dead_code)]
#[cfg(feature = "patch")]
pub async fn create_patch_file(old_file_path: impl AsRef<Path>, new_file_path: impl AsRef<Path>, patch_file_path: impl AsRef<Path>, avg_size: u32) -> Result<(), tokio::io::Error> {
    let min_size = avg_size / 4;
    let max_size = avg_size * 4;

    let new_file = File::open(new_file_path).await?;
    let mut chunks = AsyncStreamCDC::new(new_file, min_size, avg_size, max_size);
    let mut new_chunk_stream = Box::pin(chunks.as_stream());

    let old_chunks = get_file_chunks(old_file_path, min_size, avg_size, max_size).await?;

    let mut patch_file = File::create(patch_file_path).await?;
    patch_file.write_u32_le(max_size).await?;

    loop {
        let new_chunk = new_chunk_stream.next().await.and_then(|result| {
            result.ok()
        });

        if new_chunk.is_none() {
            break;
        }

        let new_chunk = new_chunk.unwrap();

        let hash = xxh64(&new_chunk.data, 0);
        let old_chunk = old_chunks.get(&hash).and_then(|chunks| {
            return chunks.get(&new_chunk.length);
        });

        match old_chunk {
            None => {
                patch_file.write_u8(1).await?;
                patch_file.write_u32_le(new_chunk.length as u32).await?;
                patch_file.write_all(&new_chunk.data).await?;
            }
            Some(old_chunk) => {
                patch_file.write_u8(0).await?;
                patch_file.write_u64_le(old_chunk.offset).await?;
                patch_file.write_u32_le(old_chunk.length as u32).await?;
            }
        }
    }

    patch_file.flush().await?;
    Ok(())
}

#[cfg(feature = "patch")]
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

#[cfg(test)]
mod test {
    use std::path::Path;
    use tokio::time::Instant;
    use crate::file_verify::calculate_file_xxhash;
    use crate::patch::file_patch::{create_patch_file, patch};

    #[tokio::test]
    async fn test_calculate_file() {
        let old_file_path = Path::new("res/Pack1.prefab");
        let new_file_path = Path::new("res/Pack2.prefab");
        let patch_file_path = Path::new("res/Pack2.patch");
        let save_file_path = Path::new("res/Pack2_new.prefab");

        let time = Instant::now();
        if let Err(e) = create_patch_file(old_file_path, new_file_path, patch_file_path, 1024 * 2).await {
            println!("{}", e);
        }
        println!("{}", time.elapsed().as_secs());
        let time = Instant::now();
        if let Err(e) = patch(old_file_path, patch_file_path, save_file_path).await {
            println!("{}", e);
        }
        println!("{}", time.elapsed().as_secs());
        let hash = calculate_file_xxhash(new_file_path, 0).await;
        println!("{}", hash.unwrap());
        let hash = calculate_file_xxhash(save_file_path, 0).await;
        println!("{}", hash.unwrap());
    }
}