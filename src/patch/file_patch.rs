use std::collections::HashMap;
use std::io::SeekFrom;
use std::path::Path;
use std::pin::Pin;
use fastcdc;
use fastcdc::v2020::{AsyncStreamCDC, ChunkData, Error};
use futures::Stream;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_stream::StreamExt;
use xxhash_rust::xxh64::xxh64;

async fn get_file_chunks(file_path: impl AsRef<Path>, avg_size: u32) -> Result<HashMap<u64, HashMap<usize, ChunkData>>, tokio::io::Error> {
    let mut hashmap: HashMap<u64, HashMap<usize, ChunkData>> = HashMap::new();
    let min_size = avg_size / 4;
    let max_size = avg_size * 4;
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

pub async fn generate_patch_file(old_file_path: impl AsRef<Path>, new_file_path: impl AsRef<Path>, patch_file_path: impl AsRef<Path>, avg_size: u32) -> Result<(), tokio::io::Error> {
    let min_size = avg_size / 4;
    let max_size = avg_size * 4;

    let new_file = File::open(new_file_path).await?;
    let mut chunks = AsyncStreamCDC::new(new_file, min_size, avg_size, max_size);
    let mut new_chunk_stream = Box::pin(chunks.as_stream());

    let mut patch_file = File::create(patch_file_path).await.expect("create patch file failed");

    patch_file.write_u32_le(avg_size).await?;

    let old_chunks = get_file_chunks(old_file_path, avg_size).await?;

    loop {
        async fn get_chunk_data(stream: &mut Pin<Box<impl Stream<Item=Result<ChunkData, Error>> + Sized>>) -> Option<ChunkData> {
            let chunk = stream.next().await;
            if chunk.is_none() {
                return None;
            }
            let result = chunk.unwrap();
            if let Err(_) = result {
                return None;
            }
            return Some(result.unwrap());
        }

        let new_chunk = get_chunk_data(&mut new_chunk_stream).await;

        if new_chunk.is_none() {
            break;
        }

        let new_chunk = new_chunk.unwrap();

        let hash = xxh64(&new_chunk.data, 0);
        match old_chunks.get(&hash) {
            None => {
                patch_file.write_u8(1).await?;
                patch_file.write_u64_le(new_chunk.length as u64).await?;
                patch_file.write_all(&new_chunk.data).await?;
            }
            Some(old_chunks) => {
                match old_chunks.get(&new_chunk.length) {
                    None => {
                        patch_file.write_u8(1).await?;
                        patch_file.write_u64_le(new_chunk.length as u64).await?;
                        patch_file.write_all(&new_chunk.data).await?;
                    }
                    Some(old_chunk) => {
                        patch_file.write_u8(0).await?;
                        patch_file.write_u64_le(old_chunk.offset).await?;
                        patch_file.write_u64_le(old_chunk.length as u64).await?;

                    }
                }
            }
        }
    }
    patch_file.flush().await?;
    Ok(())
}

pub async fn patch_file(old_file_path: impl AsRef<Path>, patch_file_path: impl AsRef<Path>, new_file_path: impl AsRef<Path>) -> Result<(), tokio::io::Error> {
    let mut new_file = File::create(new_file_path).await?;
    let mut old_file = File::open(old_file_path).await?;
    let mut patch_file = File::open(patch_file_path).await?;
    let avg_chunk_size = patch_file.read_u32_le().await? as usize;
    let mut buffer = vec![0_u8; avg_chunk_size * 4usize];
    loop {
        let bytes_read = patch_file.read(&mut buffer[0..1]).await?;
        if bytes_read == 0 {
            break;
        }
        let slice = &mut buffer[0..1];
        let op = u8::from_ne_bytes(slice.try_into().unwrap());
        if op == 0 {
            let slice = &mut buffer[0..8];
            patch_file.read_exact(slice).await?;
            let offset = u64::from_le_bytes(slice.try_into().unwrap());
            patch_file.read_exact(slice).await?;
            let length = u64::from_le_bytes(slice.try_into().unwrap()) as usize;
            old_file.seek(SeekFrom::Start(offset)).await?;
            old_file.read_exact(&mut buffer[0..length]).await?;
            new_file.write_all(&mut buffer[0..length]).await?;
        } else {
            let slice = &mut buffer[0..8];
            patch_file.read_exact(slice).await?;
            let length = u64::from_le_bytes(slice.try_into().unwrap()) as usize;
            patch_file.read_exact(&mut buffer[0..length]).await?;
            new_file.write_all(&mut buffer[0..length]).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::file_verify::calculate_file_xxhash;
    use crate::patch::file_patch::{generate_patch_file, patch_file};

    #[tokio::test]
    async fn test_calculate_file() {
        if let Err(e) = generate_patch_file("res/sausageclub_old.unity3d", "res/sausageclub_new.unity3d", "res/sausageclub.patch", 1024 * 1024).await {
            println!("{}", e);
        }
        if let Err(e) = patch_file("res/sausageclub_old.unity3d", "res/sausageclub.patch", "res/sausageclub.unity3d").await {
            println!("{}", e);
        }
        let hash = calculate_file_xxhash("res/sausageclub_new.unity3d", 0).await;
        println!("{}", hash.unwrap());
        let hash = calculate_file_xxhash("res/sausageclub.unity3d", 0).await;
        println!("{}", hash.unwrap());
    }
}