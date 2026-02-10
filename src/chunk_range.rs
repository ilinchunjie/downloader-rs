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
            let end_position = if index == chunk_count - 1 {
                total_size - 1
            } else {
                start_position + chunk_size - 1
            };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_chunk() {
        let ranges = ChunkRange::from_chunk_count(1000, 1, 500);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start, 0);
        assert_eq!(ranges[0].end, 999);
        assert_eq!(ranges[0].chunk_length(), 1000);
    }

    #[test]
    fn test_exact_multiple_of_chunk_size() {
        // 10MB file, 5MB chunk_size → 2 chunks
        let total_size = 10 * 1024 * 1024;
        let chunk_size = 5 * 1024 * 1024;
        let ranges = ChunkRange::from_chunk_count(total_size, 2, chunk_size);
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].start, 0);
        assert_eq!(ranges[0].end, chunk_size - 1);
        assert_eq!(ranges[0].chunk_length(), chunk_size);
        assert_eq!(ranges[1].start, chunk_size);
        assert_eq!(ranges[1].end, total_size - 1);
        assert_eq!(ranges[1].chunk_length(), chunk_size);
    }

    #[test]
    fn test_non_divisible_size() {
        // 11MB file, 5MB chunk_size → 3 chunks (5 + 5 + 1)
        let total_size = 11 * 1024 * 1024;
        let chunk_size = 5 * 1024 * 1024;
        let ranges = ChunkRange::from_chunk_count(total_size, 3, chunk_size);
        assert_eq!(ranges.len(), 3);
        assert_eq!(ranges[0].start, 0);
        assert_eq!(ranges[0].end, chunk_size - 1);
        assert_eq!(ranges[1].start, chunk_size);
        assert_eq!(ranges[1].end, 2 * chunk_size - 1);
        assert_eq!(ranges[2].start, 2 * chunk_size);
        assert_eq!(ranges[2].end, total_size - 1);
        // All bytes should be covered
        let total_covered: u64 = ranges.iter().map(|r| r.chunk_length()).sum();
        assert_eq!(total_covered, total_size);
    }

    #[test]
    fn test_zero_size() {
        let ranges = ChunkRange::from_chunk_count(0, 1, 1024);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start, 0);
        assert_eq!(ranges[0].end, 0);
    }

    #[test]
    fn test_eof() {
        let mut range = ChunkRange::from_start_end(100, 199);
        assert!(!range.eof());
        range.set_position(200);
        assert!(range.eof());
    }

    #[test]
    fn test_chunk_length() {
        let range = ChunkRange::from_start_end(0, 99);
        assert_eq!(range.chunk_length(), 100);
        let range = ChunkRange::from_start_end(100, 199);
        assert_eq!(range.chunk_length(), 100);
    }
}