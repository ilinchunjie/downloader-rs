mod download_handle;
mod download_task;
mod stream;
mod downloader;
mod remote_file;
mod chunk;

mod chunk_metadata;
mod chunk_hub;
mod download_configuration;
mod download_service;
mod c;
mod download_handle_file;
mod download_handle_memory;

#[cfg(test)]
mod test {
    use std::io::{Cursor, Read, Write};

    #[test]
    fn test_cursor() {
        let mut cursor = Cursor::new(Vec::new());
        let bytes: Vec<u8> = vec![1, 0, 2, 4];
        cursor.write(&bytes).unwrap();

        cursor.set_position(0);
        let mut temp_bytes: Vec<u8> = Vec::with_capacity(4);
        cursor.read_exact(&mut temp_bytes).unwrap();
        println!("{}", temp_bytes[3]);
    }
}