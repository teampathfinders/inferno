use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use paste::paste;

use crate::{BlockPosition, Deserialize, Vector};

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
pub trait BinaryRead<'a>: AsRef<[u8]> {
    declare_primitive_fns!(u16, i16, u32, i32, u64, i64, u128, i128, f32, f64);

    /// Consumes `n` bytes.
    fn advance(&mut self, n: usize) -> anyhow::Result<()>;

    /// Returns the amount of bytes remaining in the reader.
    fn remaining(&mut self) -> usize;

    /// Whether the end of the reader has been reached.
    fn eof(&mut self) -> bool {
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

    /// Reads a varuint32-prefixed slice of type `T`.
    fn read_slice<T: Deserialize<'a>>(&mut self) -> anyhow::Result<Vec<T>>;

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

    /// Reads a little endian `u24` from the reader without advancing the cursor.
    #[inline]
    fn read_u24_le(&mut self) -> anyhow::Result<u32> {
        let bytes = self.take_const::<3>()?;
        let val = u32::from_le_bytes([0, bytes[0], bytes[1], bytes[2]]);
        
        Ok(val)
    }

    /// Reads a big endian `u24` from the reader.
    #[inline]
    fn read_u24_be(&mut self) -> anyhow::Result<u32> {
        let bytes = self.take_const::<3>()?;
        let val = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], 0]);

        Ok(val)
    }

    /// Reads a little endian `u24` from the reader without advancing the cursor.
    #[inline]
    fn peek_u24_le(&self) -> anyhow::Result<u32> {
        let bytes = self.peek_const::<3>()?;
        let val = u32::from_le_bytes([0, bytes[0], bytes[1], bytes[2]]);

        Ok(val)
    }

    /// Reads a big endian `u24` from the reader without advancing the cursor.
    #[inline]
    fn peek_u24_be(&self) -> anyhow::Result<u32> {
        let bytes = self.peek_const::<3>()?;
        let val = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], 0]);

        Ok(val)
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

        bail!(Malformed, "variable 32-bit integer did not end after 5 bytes")
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

        bail!(Malformed, "variable 64-bit integer did not end after 10 bytes")
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
                bail!(Malformed, "Invalid IP type {variant}, expected either 4 or 6");
            }
        })
    }
}

impl<'a> BinaryRead<'a> for &'a [u8] {
    fn advance(&mut self, n: usize) -> anyhow::Result<()> {
        if self.len() < n {
            bail!(UnexpectedEof, "cannot advance past {n} bytes, remaining: {}", self.len())
        }

        let (_, b) = self.split_at(n);
        *self = b;

        Ok(())
    }

    #[inline]
    fn remaining(&mut self) -> usize {
        self.len()
    }

    /// Takes a specified amount of bytes from the buffer.
    ///
    /// If the amount of bytes to take from the buffer is known at compile-time,
    /// [`take_const`](BinaryRead::take_const) can be used instead.
    ///
    /// # Errors
    /// Returns [`UnexpectedEof`](crate::ErrorKind::UnexpectedEof) if the read exceeds the buffer length.
    #[inline]
    fn take_n(&mut self, n: usize) -> anyhow::Result<&'a [u8]> {
        if self.len() < n {
            crate::bail!(UnexpectedEof, "expected {n} remaining bytes, got {}", self.len())
        } else {
            let (a, b) = self.split_at(n);
            // *self = SharedBuffer::from(b);
            *self = b;
            Ok(a)
        }
    }

    /// Takes a specified amount of bytes from the buffer.
    ///
    /// This method is generic over the amount of bytes to take.
    /// In case the amount is known at compile time, this function can be used to
    /// take a sized array from the buffer.
    ///
    /// See [`take_n`](BinaryRead::take_n) for a runtime-sized alternative.
    ///
    /// # Errors
    /// Returns [`UnexpectedEof`](crate::ErrorKind::UnexpectedEof) if the read exceeds the buffer length.
    #[inline]
    fn take_const<const N: usize>(&mut self) -> anyhow::Result<[u8; N]> {
        if self.len() < N {
            bail!(UnexpectedEof, "expected {N} remaining bytes, got {}", self.len())
        } else {
            let (a, b) = self.split_at(N);
            // *self = SharedBuffer::from(b);
            *self = b;
            // SAFETY: We can unwrap because the array is guaranteed to be the required size.
            unsafe { Ok(a.try_into().unwrap_unchecked()) }
        }
    }

    /// Takes a specified amount of bytes from the buffer without advancing the cursor.
    ///
    /// If the amount of bytes to take from the buffer is known at compile-time,
    /// [`peek_const`](BinaryRead::peek_const) can be used instead.
    ///
    /// # Errors
    /// Returns [`UnexpectedEof`](crate::ErrorKind::UnexpectedEof) if the read exceeds the buffer length.
    #[inline]
    fn peek(&self, n: usize) -> anyhow::Result<&[u8]> {
        if self.len() < n {
            bail!(UnexpectedEof, "expected {n} remaining bytes, got {}", self.len())
        } else {
            Ok(&self[..n])
        }
    }

    /// Takes a specified amount of bytes from the buffer.
    ///
    /// This method is generic over the amount of bytes to take.
    /// In case the amount is known at compile time, this function can be used to
    /// take a sized array from the buffer.
    ///
    /// See [`peek`](BinaryRead::peek) for a runtime-sized alternative.
    ///
    /// # Errors
    /// Returns [`UnexpectedEof`](crate::ErrorKind::UnexpectedEof) if the read exceeds the buffer length.
    #[inline]
    fn peek_const<const N: usize>(&self) -> anyhow::Result<[u8; N]> {
        if self.len() < N {
            bail!(UnexpectedEof, "expected {N} remaining bytes, got {}", self.len())
        } else {
            let dst = &self[..N];
            // SAFETY: dst is guaranteed to be of length N
            // due to the slicing above which already implements bounds checks.
            unsafe { Ok(dst.try_into().unwrap_unchecked()) }
        }
    }

    fn read_slice<T: Deserialize<'a>>(&mut self) -> anyhow::Result<Vec<T>> {
        let len = self.read_var_u32()?;
        let mut vec = Vec::with_capacity(len as usize);

        for _ in 0..len {
            vec.push(T::deserialize_from(self)?);
        }

        Ok(vec)
    }
}

// impl<'a, R: BinaryRead<'a>> BinaryRead<'a> for &'a mut R {
//     #[inline]
//     fn advance(&mut self, n: usize) -> anyhow::Result<()> {
//         (*self).advance(n)
//     }

//     #[inline]
//     fn remaining(&self) -> usize {
//         (**self).remaining()
//     }

//     #[inline]
//     fn take_n(&mut self, n: usize) -> anyhow::Result<&'a [u8]> {
//         (*self).take_n(n)
//     }

//     #[inline]
//     fn take_const<const N: usize>(&mut self) -> anyhow::Result<[u8; N]> {
//         (*self).take_const()
//     }

//     #[inline]
//     fn peek(&self, n: usize) -> anyhow::Result<&[u8]> {
//         (**self).peek(n)
//     }

//     #[inline]
//     fn peek_const<const N: usize>(&self) -> anyhow::Result<[u8; N]> {
//         (**self).peek_const()
//     }

//     fn read_slice<T: Deserialize<'a>>(&mut self) -> anyhow::Result<Vec<T>> {
//         (**self).read_slice()
//     }
// }
