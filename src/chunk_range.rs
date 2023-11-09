#[derive(Copy, Clone)]
pub struct ChunkRange {
    pub start: u64,
    pub end: u64,
    pub position: u64,
}

impl Default for ChunkRange {
    fn default() -> Self {
        Self {
            start: 0,
            end: 0,
            position: 0,
        }
    }
}

impl ChunkRange {
    pub fn from_start_end(start: u64, end: u64) -> ChunkRange {
        ChunkRange {
            start,
            end,
            position: start,
        }
    }

    pub fn chunk_length(&self) -> u64 {
        if self.end <= self.start {
            return 0u64;
        }
        return self.end - self.start + 1;
    }

    pub fn length(&self) -> u64 {
        return self.position - self.start;
    }

    pub fn set_position(&mut self, position: u64) {
        self.position = position;
    }

    pub fn eof(&self) -> bool {
        return self.position == self.end + 1;
    }
}