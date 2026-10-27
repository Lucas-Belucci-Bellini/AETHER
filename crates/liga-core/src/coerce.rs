//! A `serde` deserializer over [`serde_json::Value`] that coerces strings into
//! numbers and booleans when the target type asks for them.
//!
//! Environment variables are always strings; this lets `PORT=3000` populate a
//! `u16` field and `DEBUG=true` populate a `bool`, while a password such as
//! `12345` still deserializes into a `String`.

use serde::de::{self, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor};
use serde::forward_to_deserialize_any;
use serde_json::{Map, Value};

pub(crate) struct ValueDe(pub(crate) Value);

type Error = serde_json::Error;

macro_rules! coerce_number {
    ($($method:ident => $ty:ty, $visit:ident;)*) => {
        $(
            fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
                if let Value::String(s) = &self.0 {
                    if let Ok(n) = s.trim().parse::<$ty>() {
                        return visitor.$visit(n);
                    }
                }
                self.deserialize_any(visitor)
            }
        )*
    };
}

impl<'de> de::Deserializer<'de> for ValueDe {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.0 {
            Value::Null => visitor.visit_unit(),
            Value::Bool(b) => visitor.visit_bool(b),
            Value::Number(n) => {
                if let Some(u) = n.as_u64() {
                    visitor.visit_u64(u)
                } else if let Some(i) = n.as_i64() {
                    visitor.visit_i64(i)
                } else if let Some(f) = n.as_f64() {
                    visitor.visit_f64(f)
                } else {
                    Err(de::Error::custom("unsupported number"))
                }
            }
            Value::String(s) => visitor.visit_string(s),
            Value::Array(items) => visitor.visit_seq(SeqDe(items.into_iter())),
            Value::Object(map) => visitor.visit_map(MapDe::new(map)),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if let Value::String(s) = &self.0 {
            match s.trim().to_ascii_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => return visitor.visit_bool(true),
                "false" | "0" | "no" | "off" => return visitor.visit_bool(false),
                _ => {}
            }
        }
        self.deserialize_any(visitor)
    }

    coerce_number! {
        deserialize_i8 => i8, visit_i8;
        deserialize_i16 => i16, visit_i16;
        deserialize_i32 => i32, visit_i32;
        deserialize_i64 => i64, visit_i64;
        deserialize_i128 => i128, visit_i128;
        deserialize_u8 => u8, visit_u8;
        deserialize_u16 => u16, visit_u16;
        deserialize_u32 => u32, visit_u32;
        deserialize_u64 => u64, visit_u64;
        deserialize_u128 => u128, visit_u128;
        deserialize_f32 => f32, visit_f32;
        deserialize_f64 => f64, visit_f64;
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.0 {
            Value::Number(n) => visitor.visit_string(n.to_string()),
            Value::Bool(b) => visitor.visit_string(b.to_string()),
            other => ValueDe(other).deserialize_any(visitor),
        }
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_string(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.0 {
            Value::Null => visitor.visit_none(),
            other => visitor.visit_some(ValueDe(other)),
        }
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        match self.0 {
            Value::String(s) => visitor.visit_enum(s.into_deserializer()),
            other => de::Deserializer::deserialize_enum(other, name, variants, visitor),
        }
    }

    forward_to_deserialize_any! {
        char bytes byte_buf unit unit_struct seq tuple tuple_struct map struct
        identifier ignored_any
    }
}

struct SeqDe(std::vec::IntoIter<Value>);

impl<'de> SeqAccess<'de> for SeqDe {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        match self.0.next() {
            Some(value) => seed.deserialize(ValueDe(value)).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.0.len())
    }
}

struct MapDe {
    iter: serde_json::map::IntoIter,
    value: Option<Value>,
}

impl MapDe {
    fn new(map: Map<String, Value>) -> Self {
        Self {
            iter: map.into_iter(),
            value: None,
        }
    }
}

impl<'de> MapAccess<'de> for MapDe {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Error> {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(ValueDe(Value::String(key))).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Error> {
        match self.value.take() {
            Some(value) => seed.deserialize(ValueDe(value)),
            None => Err(de::Error::custom("value requested before key")),
        }
    }
}
