#![allow(unused)]
#![deny(clippy::undocumented_unsafe_blocks)]

#[cfg(target_endian = "big")]
compile_error!("Big endian architectures are not supported");

use util::bytes::{BinaryRead, BinaryWrite};

/// Performs ceiling division on two u32s.
#[inline(always)]
const fn ceil_div(lhs: u32, rhs: u32) -> u32 {
    (lhs + rhs - 1) / rhs
}

/// Data returned by [`deserialize_packed_array`].
#[derive(Debug, PartialEq, Eq)]
pub enum PackedArrayReturn {
    /// The packed array contained no data.
    ///
    /// This is used by biomes that only contain a single biome type.
    Empty,
    /// The packed array specified that it inherits from a previous block.
    ///
    /// This is used by biomes. Instead of providing multiple copies of the same data,
    /// a biome can inherit from the previous entry.
    Inherit,
    /// The packed array contains new data.
    Data(Box<[u16; 4096]>),
}

/// Serialises a packed array.
#[inline(always)]
pub fn serialize_packed_array<W>(writer: &mut W, array: &[u16; 4096], max_index: usize, is_network: bool) -> anyhow::Result<()>
where
    W: BinaryWrite,
{
    // Determine the required bits per index
    let index_size = {
        let mut bits_per_block = 0;
        // Loop over allowed values.
        for b in [1, 2, 3, 4, 5, 6, 8, 16] {
            if 2usize.pow(b) >= max_index {
                bits_per_block = b;
                break;
            }
        }

        bits_per_block as u8
    };

    writer.write_u8(index_size << 1 | is_network as u8)?;

    // Amount of indices that fit in a single 32-bit integer.
    let per_word = u32::BITS / index_size as u32;

    let mut offset = 0;
    while offset < 4096 {
        let mut word = 0;
        for w in 0..per_word {
            if offset == 4096 {
                break;
            }

            let index = array[offset] as u32;
            word |= index << (w * index_size as u32);
            //            println!("word {word:#033b}, index {index:#05b}, is {index_size}");

            offset += 1;
        }

        writer.write_u32_le(word)?;
    }

    Ok(())
}

#[inline(always)]
pub fn deserialize_packed_array<'a, R>(reader: &mut R) -> anyhow::Result<PackedArrayReturn>
where
    R: BinaryRead<'a>,
{
    let index_size = reader.read_u8()? >> 1;
    if index_size == 0 {
        return Ok(PackedArrayReturn::Empty);
    } else if index_size == 0x7f {
        return Ok(PackedArrayReturn::Inherit);
    } else if ![1, 2, 3, 4, 5, 6, 8, 16].contains(&index_size) {
        anyhow::bail!(format!("Invalid index size: {index_size}"));
    }

    let per_word = u32::BITS / index_size as u32;
    let word_count = ceil_div(4096, per_word);
    let mask = !(!0u32 << index_size);

    let mut indices = Box::new([0u16; 4096]);
    let mut offset = 0;

    for _ in 0..word_count {
        let mut word = reader.read_u32_le()?;

        for _ in 0..per_word {
            if offset == 4096 {
                break;
            }

            indices[offset] = (word & mask) as u16;
            word >>= index_size;

            offset += 1;
        }
    }

    Ok(PackedArrayReturn::Data(indices))
}

#[cfg(test)]
mod test;

mod biome;
pub mod database;
mod ffi;
mod key;
pub mod provider;
mod settings;
mod sub_chunk;

pub use biome::*;
pub use key::*;
pub use sub_chunk::*;
