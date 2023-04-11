use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use paste::paste;

use crate::BlockPosition;
use crate::{u24::u24, Vector};

/// Implements the read functions for integer primitives.
macro_rules! declare_primitive_fns {
    ($($ty: ident),+) => {
        paste! {$(
            #[doc = concat!("Reads a little endian [`", stringify!($ty), "`] from the reader")]
            #[inline]
            fn [<read_ $ty _le>] (&mut self) -> anyhow::Result<$ty> {
                let bytes = self.take_const()?;
                Ok(<$ty>::from_le_bytes(bytes))
            }

            #[doc = concat!("Reads a big endian [`", stringify!($ty), "`] from the reader")]
            #[inline]
            fn [<read_ $ty _be>] (&mut self) -> anyhow::Result<$ty> {
                let bytes = self.take_const()?;
                Ok(<$ty>::from_be_bytes(bytes))
            }

            #[doc = concat!("Reads a little endian [`", stringify!($ty), "`] from the reader without advancing the cursor")]
            #[inline]
            fn [<peek_ $ty _le>](&self) -> anyhow::Result<$ty> {
                let bytes = self.peek_const()?;
                Ok(<$ty>::from_le_bytes(bytes))
            }

            #[doc = concat!("Reads a big endian [`", stringify!($ty), "`] from the reader without advancing the cursor")]
            #[inline]
            fn [<peek_ $ty _be>](&self) -> anyhow::Result<$ty> {
                let bytes = self.peek_const()?;
                Ok(<$ty>::from_be_bytes(bytes))
            }
        )+}
    }
}

/// Adds binary reading capabilities to a reader.
pub trait BinaryRead<'a> {
    declare_primitive_fns!(
        u16, i16, u24, u32, i32, u64, i64, u128, i128, f32, f64
    );

    /// Consumes `n` bytes.
    fn advance(&mut self, n: usize) -> anyhow::Result<()>;

    /// Returns the amount of bytes remaining in the reader.
    fn remaining(&self) -> usize;

    /// Whether the end of the reader has been reached.
    fn eof(&self) -> bool {
        self.remaining() == 0
    }

    /// Takes `n` bytes out of the reader.
    fn take_n(&mut self, n: usize) -> anyhow::Result<&'a [u8]>;
    /// Takes `N` bytes out of the reader.
    /// This can be used to get sized arrays if the size is known at compile time.
    fn take_const<const N: usize>(&mut self) -> anyhow::Result<[u8; N]>;
    /// Takes `n` bytes out of the reader without advancing the cursor.
    fn peek(&self, n: usize) -> anyhow::Result<&[u8]>;
    /// Takes `N` bytes out of the reader without advancing the cursor.
    /// /// This can be used to get sized arrays if the size is known at compile time.
    fn peek_const<const N: usize>(&self) -> anyhow::Result<[u8; N]>;

    /// Reads a [`bool`] from the reader.
    #[inline]
    fn read_bool(&mut self) -> anyhow::Result<bool> {
        Ok(self.take_const::<1>()?[0] != 0)
    }

    /// Reads a [`u8`] from the reader.
    #[inline]
    fn read_u8(&mut self) -> anyhow::Result<u8> {
        Ok(self.take_const::<1>()?[0])
    }

    /// Reads an [`i8`] from the reader.
    #[inline]
    fn read_i8(&mut self) -> anyhow::Result<i8> {
        Ok(self.take_const::<1>()?[0] as i8)
    }

    /// Reads a variable size [`u32`] from the reader.
    #[inline]
    fn read_var_u32(&mut self) -> anyhow::Result<u32> {
        let mut v = 0;
        let mut i = 0;
        while i < 35 {
            let b = self.read_u8()?;
            v |= ((b & 0x7f) as u32) << i;
            if b & 0x80 == 0 {
                return Ok(v);
            }
            i += 7;
        }

        anyhow::bail!("Variable 32-bit integer did not end after 5 bytes")
    }

    /// Reads a variable size [`i32`] from the reader.
    #[inline]
    fn read_var_u64(&mut self) -> anyhow::Result<u64> {
        let mut v = 0;
        let mut i = 0;
        while i < 70 {
            let b = self.read_u8()?;
            v |= ((b & 0x7f) as u64) << i;
            if b & 0x80 == 0 {
                return Ok(v);
            }
            i += 7;
        }

        anyhow::bail!("Variable 64-bit integer did not end after 10 bytes")
    }

    /// Reads a variable size [`u64`] from the reader.
    #[inline]
    fn read_var_i32(&mut self) -> anyhow::Result<i32> {
        let vx = self.read_var_u32()?;
        let mut v = (vx >> 1) as i32;

        if vx & 1 != 0 {
            v = !v;
        }

        Ok(v)
    }

    /// Reads a variable size [`i64`] from the reader.
    #[inline]
    fn read_var_i64(&mut self) -> anyhow::Result<i64> {
        let vx = self.read_var_u64()?;
        let mut v = (vx >> 1) as i64;

        if vx & 1 != 0 {
            v = !v;
        }

        Ok(v)
    }

    /// Reads a string prefixed by a variable u32.
    #[inline]
    fn read_str(&mut self) -> anyhow::Result<&'a str> {
        let len = self.read_var_u32()?;
        let data = self.take_n(len as usize)?;

        Ok(std::str::from_utf8(data)?)
    }

    #[inline]
    fn read_block_pos(&mut self) -> anyhow::Result<BlockPosition> {
        let x = self.read_var_i32()?;
        let y = self.read_var_u32()?;
        let z = self.read_var_i32()?;

        Ok(BlockPosition::new(x, y, z))
    }

    /// Reads a byte vector from the buffer.
    #[inline]
    fn read_vecb<const N: usize>(&mut self) -> anyhow::Result<Vector<i8, N>> {
        let mut x = [0; N];
        for v in &mut x {
            *v = self.read_i8()?;
        }
        Ok(Vector::from(x))
    }

    /// Reads an integer vector from the buffer.
    #[inline]
    fn read_veci<const N: usize>(&mut self) -> anyhow::Result<Vector<i32, N>> {
        let mut x = [0; N];
        for v in &mut x {
            *v = self.read_var_i32()?;
        }
        Ok(Vector::from(x))
    }

    /// Reads a float vector from the buffer.
    #[inline]
    fn read_vecf<const N: usize>(&mut self) -> anyhow::Result<Vector<f32, N>> {
        let mut x = [0.0; N];
        for v in &mut x {
            *v = self.read_f32_le()?;
        }
        Ok(Vector::from(x))
    }

    /// Reads an IP address from the buffer.
    fn read_addr(&mut self) -> anyhow::Result<SocketAddr> {
        let variant = self.read_u8()?;
        Ok(match variant {
            4 => {
                let addr = IpAddr::V4(Ipv4Addr::from(self.read_u32_be()?));
                let port = self.read_u16_be()?;

                SocketAddr::new(addr, port)
            }
            6 => {
                self.advance(2)?; // IP family (AF_INET6)
                let port = self.read_u16_be()?;
                self.advance(4)?; // Flow information
                let addr = IpAddr::V6(Ipv6Addr::from(self.read_u128_be()?));
                self.advance(4)?; // Scope ID

                SocketAddr::new(addr, port)
            }
            _ => {
                anyhow::bail!(format!(
                    "Invalid IP type {variant}, expected either 4 or 6"
                ));
            }
        })
    }
}

impl<'a, R: BinaryRead<'a>> BinaryRead<'a> for &'a mut R {
    #[inline]
    fn advance(&mut self, n: usize) -> anyhow::Result<()> {
        (*self).advance(n)
    }

    #[inline]
    fn remaining(&self) -> usize {
        (**self).remaining()
    }

    #[inline]
    fn take_n(&mut self, n: usize) -> anyhow::Result<&'a [u8]> {
        (*self).take_n(n)
    }

    #[inline]
    fn take_const<const N: usize>(&mut self) -> anyhow::Result<[u8; N]> {
        (*self).take_const()
    }

    #[inline]
    fn peek(&self, n: usize) -> anyhow::Result<&[u8]> {
        (**self).peek(n)
    }

    #[inline]
    fn peek_const<const N: usize>(&self) -> anyhow::Result<[u8; N]> {
        (**self).peek_const()
    }
}
