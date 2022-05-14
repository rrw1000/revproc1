use async_trait::async_trait;
use std::{cell::RefCell, cmp, fmt};

#[async_trait]
pub trait MemoryAccess {
    async fn read(&self, loc: u64, span: u32) -> &Vec<u64>;
    async fn write(&self, loc: u64, contents: &Vec<u64>);
}

enum MemOp {
    Read,
    Write,
}

enum MemorySegment {
    Nothing(),
    Next(RefCell<Vec<MemorySegment>>),
    Memory(RefCell<Vec<u64>>),
}

impl fmt::Debug for MemorySegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dbg = f.debug_struct("MemorySegment");
        match self {
            MemorySegment::Nothing() => {
                dbg.field("Nothing", &"nothing");
                ()
            }
            MemorySegment::Next(segment) => {
                let mut idx = 0;
                for mem in &*segment.borrow() {
                    dbg.field("Seg{i}", &mem);
                    idx += 1;
                }
            }
            MemorySegment::Memory(vec) => {
                dbg.field("Memory", &vec.borrow().len());
                ()
            }
        }
        dbg.finish()
    }
}

/// A tree memory, populated on demand.
/// Each level of the tree supplies bits_per_segment bits, and each
/// element is either a memory array, a pointer to another table, or nothing.
/// By default we put memory itself at the leaves.
pub struct TreeMemory {
    /// Root of the tree.
    root: RefCell<MemorySegment>,
    /// Bits per segment - size of the tables
    bits_per_segment: u32,
    /// How deep is the tree?
    max_depth: u32,
}

impl TreeMemory {
    pub fn new() -> TreeMemory {
        TreeMemory {
            root: RefCell::new(MemorySegment::Nothing()),
            // 4096 element per array
            bits_per_segment: 12,
            // 48 bits => 1MiB segments - a bit small, but ...
            max_depth: 4,
        }
    }

    /// Perform an iop against a segment, with a base address and depth
    fn iop(
        &self,
        offset: u64,
        iovec: &mut Vec<u64>,
        op: MemOp,
        segment: &RefCell<MemorySegment>,
        address: u64,
        depth: u32,
    ) {
    }

    fn get_max_address(&self, address: u64, depth: u32) -> u64 {
        // Compute the base and limit of this segment
        let bits_in_address = self.bits_per_segment * depth;
        let mask = ((1 << bits_in_address) - 1) << (64 - bits_in_address);
        address | (!0 & mask)
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::memory;
    #[test]
    fn create_memory() {
        let mem = memory::TreeMemory::new();
        assert!(matches!(
            *mem.root.borrow(),
            memory::MemorySegment::Nothing()
        ));
    }
}
