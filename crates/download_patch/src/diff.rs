use std::collections::HashMap;
use std::path::Path;
use fastcdc::v2020::{AsyncStreamCDC, ChunkData};
use tokio::fs::File;
use tokio::io::{AsyncWriteExt};
use tokio_stream::StreamExt;
use xxhash_rust::xxh64::xxh64;

#[allow(dead_code)]
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