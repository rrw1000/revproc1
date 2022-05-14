use async_trait::async_trait;
use std::{cmp, vec::Vec};

#[async_trait]
pub trait MemoryAccess {
    async fn read(&self, loc: u64, span: u32) -> &Vec<u64>;
    async fn write(&self, loc: u64, contents: &Vec<u64>);
}

struct MemorySegment {
    start_address: u64,
    contents: Vec<u64>,
}

impl MemorySegment {
    // Return 1 past the end of the segment.
    fn limit(&self) -> u64 {
        self.start_address + (self.contents.len() as u64)
    }
    fn bytes(&self) -> u64 {
        self.contents.len() as u64
    }
}

pub struct VectorMemory {
    /// Values not in a segment read as 0.
    segments: Vec<MemorySegment>,

    /// Segment size, in log_2 words (so 1MW == 20)
    chunk_size: u64,
}

enum MemOp {
    Read,
    Write,
}

impl VectorMemory {
    pub fn new() -> VectorMemory {
        // 1MW chunks (== 16MiB)
        VectorMemory {
            segments: vec![],
            chunk_size: 20,
        }
    }

    /// Ensure that chunks are present for a segment from loc to size
    async fn ensure_present(&mut self, loc: u64, size: u64) {
        // Generate a list of segments that should be there.
        let segs: Vec<u64> = vec![];
        let first = loc >> chunk_size;
        let last = ((loc + size) >> chunk_size) + 1;
    }

    /// Read from or write to memory.
    /// Updates elements of `iovec` and `self.segments` according to
    /// the operation and addresses.
    /// Out of range addresses are not touched.
    async fn iop(&mut self, loc: u64, iovec: &mut Vec<u64>, op: MemOp) {
        let limit = loc + (iovec.len() as u64);
        for segment in &mut self.segments {
            let segment_limit = segment.limit();
            let copy_start_loc = cmp::max(segment.start_address, loc);
            let copy_end_loc = cmp::min(segment.limit(), limit);
            if copy_start_loc > segment_limit {
                continue;
            }
            if copy_end_loc < segment.start_address {
                continue;
            }
            let copy_mem_offset = copy_start_loc - segment.start_address;
            let copy_vec_offset = copy_start_loc - loc;
            let copy_nr_bytes = cmp::min(copy_end_loc - copy_start_loc, segment.bytes() as u64);
            let mem = &mut segment.contents
                [(copy_mem_offset as usize)..((copy_mem_offset + copy_nr_bytes) as usize)];
            let iov = &mut iovec
                [(copy_vec_offset as usize)..((copy_vec_offset + copy_nr_bytes) as usize)];
            // outside the map for efficiency
            match op {
                MemOp::Read => {
                    for (mem_i, iov_i) in mem.into_iter().zip(iov.iter_mut()) {
                        *iov_i = *mem_i;
                    }
                }
                MemOp::Write => {
                    for (mem_i, iov_i) in mem.iter_mut().zip(iov.into_iter()) {
                        *mem_i = *iov_i;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::memory;
    #[test]
    fn create_vector_memory() {
        let mem = memory::VectorMemory::new();
        assert_eq!(mem.segments.len(), 0);
    }
}
