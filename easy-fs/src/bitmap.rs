use alloc::sync::Arc;

use crate::{block_cache::get_block_cache, block_dev::BlockDevice, BLOCK_SZ};

type BitmapBlock = [u64; 64];
const BLOCK_BITS: usize = BLOCK_SZ * 8;

/// Return (block_pos, bits64_pos, inner_pos)
/// means: the nth of block, the nth of u64, the nth of bit in u64
fn decomposition(mut bit: usize) -> (usize, usize, usize) {
    let block_pos = bit / BLOCK_BITS;
    bit = bit % BLOCK_BITS;
    (block_pos, bit / 64, bit % 64)
}

/// Bitmap responsible for handle the bitmap blocks
pub struct Bitmap {
    start_block_id: usize,
    blocks: usize,
}

impl Bitmap {
    pub fn new(start_block_id: usize, blocks: usize) -> Self {
        Self {
            start_block_id,
            blocks,
        }
    }

    // find the first 0 bit in the bitmap blocks, set it to 1, return the bit position
    pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
        for block_id in 0..self.blocks {
            let pos = get_block_cache(block_id + self.start_block_id, Arc::clone(block_device))
                .lock()
                .modify(0, |bitmap_block: &mut BitmapBlock| {
                    // read block as bitmap block, find the first bit that is 0
                    if let Some((bits64_pos, inner_pos)) = bitmap_block
                        .iter()
                        .enumerate()
                        .find(|(_, bits64)| **bits64 != u64::MAX)
                        .map(|(bits64_pos, bits64)| (bits64_pos, bits64.trailing_ones()))
                    // the number of trailling ones is the first 0 bit position
                    {
                        // set the bit to 1
                        bitmap_block[bits64_pos] |= 1 << inner_pos;
                        // return the bit position
                        Some(block_id * BLOCK_BITS + bits64_pos * 64 + inner_pos as usize)
                    } else {
                        // all bits are 1
                        None
                    }
                });
            if pos.is_some() {
                return pos;
            }
        }
        None
    }

    pub fn dealloc(&self, block_device: &Arc<dyn BlockDevice>, bit: usize) {
        let (block_pos, bits64_pos, inner_pos) = decomposition(bit);
        // the block where the bit is located
        let cache = get_block_cache(block_pos + self.start_block_id, Arc::clone(block_device));
        cache.lock().modify(0, |bitmap_block: &mut BitmapBlock| {
            // the bit should be 1
            assert!(bitmap_block[bits64_pos] & (1 << inner_pos) != 0);
            // set the bit to 0
            bitmap_block[bits64_pos] &= !(1 << inner_pos);
        });
    }

    /// Return the maximum block number that can be represented by the bitmap
    pub fn maximum(&self) -> usize {
        self.blocks * BLOCK_BITS
    }
}
