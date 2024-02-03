#![deny(
    clippy::expect_used,
    clippy::get_unwrap,
    clippy::if_then_some_else_none,
    clippy::impl_trait_in_params,
    clippy::let_underscore_untyped,
    clippy::missing_assert_message,
    clippy::mutex_atomic,
    clippy::undocumented_unsafe_blocks,
    clippy::unwrap_in_result,
    clippy::unwrap_used,
    clippy::str_to_string,
    clippy::clone_on_ref_ptr,
    clippy::nursery,
    clippy::default_trait_access,
    clippy::doc_link_with_quotes,
    clippy::expl_impl_clone_on_copy,
    clippy::explicit_deref_methods,
    clippy::explicit_into_iter_loop,
    clippy::explicit_iter_loop,
    clippy::implicit_clone,
    clippy::index_refutable_slice,
    clippy::inefficient_to_string,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::large_stack_arrays,
    clippy::manual_instant_elapsed,
    clippy::manual_let_else,
    clippy::match_bool,
    clippy::missing_fields_in_debug,
    clippy::missing_panics_doc,
    clippy::redundant_closure_for_method_calls,
    clippy::single_match_else,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unused_self,
    clippy::unused_async
)]
#![allow(dead_code)]
#![allow(clippy::use_self)]

pub use crate::de::{from_be_bytes, from_le_bytes, from_var_bytes, Deserializer};
pub use crate::ser::{to_be_bytes, to_be_bytes_in, to_le_bytes, to_le_bytes_in, to_var_bytes, to_var_bytes_in, Serializer};
pub use crate::value::Value;
use anyhow::anyhow;
use std::fmt::{Debug, Display, Formatter};

#[cfg(test)]
mod test;

mod de;
mod ser;
mod value;

mod private {
    use crate::{BigEndian, LittleEndian, Variable};

    /// Prevents [`VariantImpl`](super::VariantImpl) from being implemented for
    /// types outside of this crate.
    pub trait Sealed {}

    impl Sealed for LittleEndian {}
    impl Sealed for BigEndian {}
    impl Sealed for Variable {}
}

/// Implemented by all NBT variants.
pub trait VariantImpl: private::Sealed {
    /// Used to convert a variant to an enum.
    /// This is used to match generic types in order to prevent
    /// having to duplicate all deserialisation code three times.
    const AS_ENUM: Variant;
}

/// NBT format variant.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Variant {
    /// Used by Bedrock for data saved to disk.
    /// Every data type is written in little endian format.
    LittleEndian,
    /// Used by Java.
    /// Every data types is written in big endian format.
    BigEndian,
    /// Used by Bedrock for NBT transferred over the network.
    /// This format is the same as [`LittleEndian`], except that type lengths
    /// (such as for strings or lists), are varints instead of shorts.
    /// The integer and long types are also varints.
    Variable,
}

/// Used by Bedrock for data saved to disk.
/// Every data type is written in little endian format.
pub enum LittleEndian {}

impl VariantImpl for LittleEndian {
    const AS_ENUM: Variant = Variant::LittleEndian;
}

/// Used by Java.
/// Every data types is written in big endian format.
pub enum BigEndian {}

impl VariantImpl for BigEndian {
    const AS_ENUM: Variant = Variant::BigEndian;
}

/// Used by Bedrock for NBT transferred over the network.
/// This format is the same as [`LittleEndian`], except that type lengths
/// (such as for strings or lists), are varints instead of shorts.
/// The integer and long types are also varints.
pub enum Variable {}

impl VariantImpl for Variable {
    const AS_ENUM: Variant = Variant::Variable;
}

/// NBT field type
// Compiler complains about unused enum variants even though they're constructed using a transmute.
#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
enum FieldType {
    /// Indicates the end of a compound tag.
    End = 0,
    /// A signed byte.
    Byte = 1,
    /// A signed short.
    Short = 2,
    /// A signed int.
    Int = 3,
    /// A signed long.
    Long = 4,
    /// A float.
    Float = 5,
    /// A double.
    Double = 6,
    /// An array of byte tags.
    ByteArray = 7,
    /// A UTF-8 string.
    String = 8,
    /// List of tags.
    /// Every item in the list must be of the same type.
    List = 9,
    /// A key-value map.
    Compound = 10,
    /// An array of int tags.
    IntArray = 11,
    /// An array of long tags.
    LongArray = 12,
}

impl TryFrom<u8> for FieldType {
    type Error = anyhow::Error;

    fn try_from(v: u8) -> anyhow::Result<Self> {
        const LAST_DISC: u8 = FieldType::LongArray as u8;
        if v > LAST_DISC {
            util::bail!(Other, "NBT field type discriminant out of range");
        }

        // SAFETY: Because `Self` is marked as `repr(u8)`, its layout is guaranteed to start
        // with a `u8` discriminant as its first field. Additionally, the raw discriminant is verified
        // to be in the enum's range.
        Ok(unsafe { std::mem::transmute::<u8, FieldType>(v) })
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct NbtError(anyhow::Error);

impl From<anyhow::Error> for NbtError {
    fn from(value: anyhow::Error) -> Self {
        Self(value)
    }
}

impl From<util::Error> for NbtError {
    fn from(value: util::Error) -> Self {
        Self(value.into())
    }
}

impl From<std::io::Error> for NbtError {
    fn from(value: std::io::Error) -> Self {
        Self(value.into())
    }
}

impl From<std::string::FromUtf8Error> for NbtError {
    fn from(value: std::string::FromUtf8Error) -> Self {
        Self(value.into())
    }
}

impl From<std::str::Utf8Error> for NbtError {
    fn from(value: std::str::Utf8Error) -> Self {
        Self(value.into())
    }
}

impl Display for NbtError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for NbtError {}

impl serde::de::Error for NbtError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Self(anyhow!(msg.to_string()))
    }
}

impl serde::ser::Error for NbtError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Self(anyhow!(msg.to_string()))
    }
}
