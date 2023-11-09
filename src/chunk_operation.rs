use crate::chunk_range::ChunkRange;

pub struct ChunkOperation {
    downloaded_size: u64,
    total_size: u64,
}

impl Default for ChunkOperation {
    fn default() -> Self {
        Self {
            downloaded_size: 0,
            total_size: 0,
        }
    }
}

impl ChunkOperation {
    pub fn with_chunk(chunk_range: &ChunkRange) -> ChunkOperation {
        ChunkOperation {
            total_size: chunk_range.chunk_length(),
            downloaded_size: chunk_range.length(),
        }
    }

    pub fn set_downloaded_size(&mut self, size: u64) {
        self.downloaded_size = size;
    }

    pub fn set_total_size(&mut self, size: u64) {
        self.total_size = size;
    }

    pub fn get_downloaded_size(&mut self) -> u64 {
        return self.downloaded_size;
    }

    pub fn get_total_size(&mut self) -> u64 {
        return self.total_size;
    }
}