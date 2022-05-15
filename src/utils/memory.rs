use async_trait::async_trait;
use std::{cell::RefCell, fmt};

#[async_trait]
pub trait Access {
    /// Read some words - you could use internal mutability here,
    /// but I think it's more honest to acknowledge that reads
    /// can change the struct (eg. by caching)
    async fn read(&mut self, loc: u64, span: u32) -> Vec<u64>;

    /// Write some words.
    async fn write(&mut self, loc: u64, contents: &Vec<u64>);

    /// Utility functions
    async fn read_64(&mut self, loc: u64) -> u64;
    async fn write_64(&mut self, loc: u64, val: u64);
}

#[derive(Debug)]
pub enum MemOp {
    Read,
    Write,
}

enum MemorySegment {
    Nothing(),
    Next(Vec<RefCell<MemorySegment>>),
    Memory(RefCell<Vec<u64>>),
}

impl MemorySegment {
    fn new_memory(mem_bits: u32) -> MemorySegment {
        MemorySegment::Memory(RefCell::new(vec![0; 1 << mem_bits]))
    }
    fn new_segment(seg_bits: u32) -> MemorySegment {
        // I don't really want to implement Copy() for MemorySegments, so ...
        let mut result: Vec<RefCell<MemorySegment>> = Vec::with_capacity((1 << seg_bits) as usize);
        for _ in 0..(1 << seg_bits) {
            result.push(RefCell::new(MemorySegment::Nothing()));
        }
        MemorySegment::Next(result)
    }
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
                for mem in segment {
                    match *mem.borrow() {
                        MemorySegment::Nothing() => (),
                        _ => {
                            dbg.field(&format!("Seg {idx}"), mem);
                            ()
                        }
                    }
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
    /// Bits in an end index - cached here for convenience
    mem_bits: u32,
}

impl TreeMemory {
    pub fn new() -> TreeMemory {
        TreeMemory {
            root: RefCell::new(MemorySegment::Nothing()),
            /// 4096 element per array
            bits_per_segment: 12,
            /// 48 bits => 1MiB segments - a bit small, but ...
            max_depth: 5,
            mem_bits: (64 - (12 * 4)),
        }
    }

    /// Perform an iop against a segment
    /// iops must be aligned within a single memory segments - splitting them
    /// happens at the cache layer (to simulate a segmented memory architecture)
    pub fn iop(&mut self, address: u64, iovec: &mut Vec<u64>, op: MemOp) {
        self.run_op(&self.root, address, iovec, &op, 1);
    }

    fn run_op(
        &self,
        parent: &RefCell<MemorySegment>,
        address: u64,
        iovec: &mut Vec<u64>,
        op: &MemOp,
        level: u32,
    ) {
        let shift = 64 - (self.bits_per_segment * level);
        let mask = (1 << self.bits_per_segment) - 1;
        let idx = (address >> shift) & mask;
        let final_idx = address & ((1 << self.mem_bits) - 1);
        // println!("level {level:x} address {address:x} shift {shift} idx {idx:x} mask {mask:x} final {final_idx:x} op {op:?}");

        //println!("run_op level {level}, op {op:?} node {parent:?}");
        let fault_in = match op {
            MemOp::Read => match &*parent.borrow() {
                MemorySegment::Nothing() => false,
                MemorySegment::Next(next_seg) => {
                    self.run_op(&next_seg[idx as usize], address, iovec, op, level + 1);
                    false
                }
                MemorySegment::Memory(mem) => {
                    let src = mem.borrow();
                    let src_iter = src[(final_idx as usize)..].into_iter();
                    let dst_iter = iovec.iter_mut();
                    for (dst_i, src_i) in dst_iter.zip(src_iter) {
                        *dst_i = *src_i;
                    }
                    false
                }
            },
            MemOp::Write => match &*parent.borrow() {
                MemorySegment::Nothing() => true,
                MemorySegment::Next(next_seg) => {
                    self.run_op(&next_seg[idx as usize], address, iovec, op, level + 1);
                    false
                }
                MemorySegment::Memory(mem) => {
                    let mut dst = mem.borrow_mut();
                    let src_iter = iovec.into_iter();
                    let dst_iter = dst[(final_idx as usize)..].iter_mut();
                    for (dst_i, src_i) in dst_iter.zip(src_iter) {
                        *dst_i = *src_i;
                    }
                    false
                }
            },
        };

        if fault_in {
            // If we get here, we are writing and need to replace parent.
            if level == self.max_depth - 1 {
                //println!("Replacing with memory");
                parent.replace(MemorySegment::new_memory(self.mem_bits));
            } else {
                //println!("Replacing with indirection");
                parent.replace(MemorySegment::new_segment(self.mem_bits));
            }
            //println!("Got {parent:?}");
            // And try again
            self.run_op(parent, address, iovec, op, level);
        }
    }
}

#[async_trait]
impl Access for TreeMemory {
    async fn read(&mut self, loc: u64, span: u32) -> Vec<u64> {
        let mut iovec = vec![0; span as usize];
        self.iop(loc, &mut iovec, MemOp::Read);
        iovec
    }

    async fn write(&mut self, loc: u64, contents: &Vec<u64>) {
        let mut a_spurious_copy = contents.clone();
        self.iop(loc, &mut a_spurious_copy, MemOp::Write);
    }

    async fn read_64(&mut self, loc: u64) -> u64 {
        self.read(loc, 1).await[0]
    }

    async fn write_64(&mut self, loc: u64, val: u64) {
        let iovec = vec![val];
        self.write(loc, &iovec).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::memory;
    #[test]
    fn create_memory() {
        let _mem = memory::TreeMemory::new();
    }

    #[test]
    fn check_io() {
        let mut mem = memory::TreeMemory::new();
        let mut some_data: Vec<u64> = vec![2, 34, 67, 0x898, 0x12345678];
        mem.iop(0, &mut some_data, MemOp::Write);
        let mut other_data: Vec<u64> = vec![0; 16];
        mem.iop(0, &mut other_data, MemOp::Read);
        assert_eq!(some_data, other_data[0..some_data.len()]);
    }

    #[tokio::test]
    async fn check_interface() {
        let mut mem = memory::TreeMemory::new();
        let mut data_out = vec![238];
        mem.iop(0, &mut data_out, MemOp::Write);
        let mut data_out2 = vec![45678];
        mem.iop(0, &mut data_out2, MemOp::Write);
        assert_eq!(mem.read_64(0).await, 45678);
        let data_out3 = vec![12345];
        mem.write(0, &data_out3).await;
        assert_eq!(mem.read_64(0).await, 12345);

        mem.write_64(0, 0x45788).await;
        let mut data_in = vec![0];
        mem.iop(0, &mut data_in, MemOp::Read);
        assert_eq!(data_in[0], 0x45788);
        assert_eq!(mem.read_64(0).await, 0x45788);
        mem.write_64(0x12345678u64, 42).await;
        assert_eq!(mem.read_64(0).await, 0x45788);
        assert_eq!(mem.read_64(0x12345678u64).await, 42);
        assert_eq!(mem.read_64(1).await, 0);
    }
}
