use crate::{RefTag, Value, TAG_BYTE, TAG_END};
use bytes::{BufMut, BytesMut};
use common::WriteExtensions;

impl RefTag<'_> {
    /// Encodes an NBT structure, returning the created buffer (little endian).
    pub fn encode_net(&self) -> Vec<u8> {
        let mut stream = BytesMut::new();
        self.encode_with_net(&mut stream);

        stream.to_vec()
    }

    /// Writes the NBT data into the given stream (little endian).
    pub fn encode_with_net(&self, stream: &mut BytesMut) {
        Value::encode_tag_net(stream, (self.name, &self.value))
    }
}

impl Value {
    /// Encodes a name-value combo (little endian).
    fn encode_tag_net(stream: &mut BytesMut, tag: (&str, &Self)) {
        stream.put_u8(tag.1.as_numeric_id());
        if matches!(tag.1, Self::End) {
            return;
        }

        Self::encode_tag_name_net(stream, tag.0);
        Self::encode_tag_value_net(stream, tag.1);
    }

    /// Encodes an NBT tag name (little endian).
    fn encode_tag_name_net(stream: &mut BytesMut, string: &str) {
        tracing::info!("{string}");

        stream.put_var_u32(string.len() as u32);
        stream.put(string.as_bytes());
    }

    /// Encodes an NBT tag value (little endian).
    fn encode_tag_value_net(stream: &mut BytesMut, value: &Self) {
        match value {
            Self::End => (),
            Self::Byte(v) => stream.put_i8(*v),
            Self::Short(v) => stream.put_i16_le(*v),
            Self::Int(v) => stream.put_var_i32(*v),
            Self::Long(v) => stream.put_var_i64(*v),
            Self::Float(v) => stream.put_f32_le(*v),
            Self::Double(v) => stream.put_f64_le(*v),
            Self::String(v) => Self::encode_tag_name_net(stream, v),
            Self::List(v) => {
                stream.put_u8(v.get(0).map(|t| t.as_numeric_id()).unwrap_or(TAG_BYTE));
                for t in v {
                    Self::encode_tag_value_net(stream, t);
                }
            }
            Self::Compound(v) => {
                for t in v.iter() {
                    Self::encode_tag_net(stream, (t.0, t.1)); // Tuple is like this to force &String to convert to &str.
                }
                stream.put_u8(TAG_END);
            }
            Self::ByteArray(v) => {
                stream.put_var_i32(v.len() as i32);
                for t in v {
                    stream.put_i8(*t);
                }
            }
            Self::IntArray(v) => {
                stream.put_var_i32(v.len() as i32);
                for t in v {
                    stream.put_var_i32(*t);
                }
            }
            Self::LongArray(v) => {
                stream.put_var_i32(v.len() as i32);
                for t in v {
                    stream.put_var_i64(*t);
                }
            }
        }
    }
}