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

    pub fn from_chunk_count(total_size: u64, chunk_count: u64, chunk_size: u64) -> Vec<ChunkRange> {
        let mut chunk_ranges: Vec<ChunkRange> = Vec::with_capacity(chunk_count as usize);
        if chunk_count == 1 {
            let mut end_position = 0;
            if total_size > 0 {
                end_position = total_size - 1;
            }
            chunk_ranges.push(ChunkRange::from_start_end(0, end_position));
            return chunk_ranges;
        }
        for index in 0..chunk_count {
            let start_position = index * chunk_size;
            let mut end_position = start_position + chunk_size - 1;
            if index == chunk_count - 1 {
                end_position = start_position + total_size % chunk_size - 1;
            }
            chunk_ranges.push(ChunkRange::from_start_end(start_position, end_position));
        }
        chunk_ranges
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