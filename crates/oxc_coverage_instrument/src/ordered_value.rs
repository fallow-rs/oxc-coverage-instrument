//! Ordered, JSON-compatible serde value used to build runtime setup AST.

use std::fmt;

use serde::{Serialize, ser};

#[derive(Debug)]
pub enum OrderedValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
}

#[derive(Debug)]
pub struct OrderedValueError(String);

impl fmt::Display for OrderedValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for OrderedValueError {}

impl ser::Error for OrderedValueError {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self(message.to_string())
    }
}

pub fn to_ordered_value<T: Serialize>(value: &T) -> Result<OrderedValue, OrderedValueError> {
    value.serialize(ValueSerializer)
}

struct ValueSerializer;

impl ser::Serializer for ValueSerializer {
    type Ok = OrderedValue;
    type Error = OrderedValueError;
    type SerializeSeq = SequenceSerializer;
    type SerializeTuple = SequenceSerializer;
    type SerializeTupleStruct = SequenceSerializer;
    type SerializeTupleVariant = TupleVariantSerializer;
    type SerializeMap = MapSerializer;
    type SerializeStruct = MapSerializer;
    type SerializeStructVariant = StructVariantSerializer;

    fn serialize_bool(self, value: bool) -> Result<Self::Ok, Self::Error> {
        Ok(OrderedValue::Bool(value))
    }

    fn serialize_i8(self, value: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(i64::from(value))
    }

    fn serialize_i16(self, value: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(i64::from(value))
    }

    fn serialize_i32(self, value: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(i64::from(value))
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "runtime coverage data is represented by JavaScript Number values"
    )]
    fn serialize_i64(self, value: i64) -> Result<Self::Ok, Self::Error> {
        Ok(OrderedValue::Number(value as f64))
    }

    fn serialize_u8(self, value: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(u64::from(value))
    }

    fn serialize_u16(self, value: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(u64::from(value))
    }

    fn serialize_u32(self, value: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(u64::from(value))
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "runtime coverage data is represented by JavaScript Number values"
    )]
    fn serialize_u64(self, value: u64) -> Result<Self::Ok, Self::Error> {
        Ok(OrderedValue::Number(value as f64))
    }

    fn serialize_f32(self, value: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(f64::from(value))
    }

    fn serialize_f64(self, value: f64) -> Result<Self::Ok, Self::Error> {
        if value.is_finite() { Ok(OrderedValue::Number(value)) } else { Ok(OrderedValue::Null) }
    }

    fn serialize_char(self, value: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(&value.to_string())
    }

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        Ok(OrderedValue::String(value.to_string()))
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(OrderedValue::Array(
            value.iter().map(|byte| OrderedValue::Number(f64::from(*byte))).collect(),
        ))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(OrderedValue::Null)
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(OrderedValue::Null)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(OrderedValue::Object(vec![(variant.to_string(), value.serialize(self)?)]))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SequenceSerializer { values: Vec::with_capacity(len.unwrap_or(0)) })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(TupleVariantSerializer {
            variant: variant.to_string(),
            sequence: SequenceSerializer { values: Vec::with_capacity(len) },
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer { entries: Vec::with_capacity(len.unwrap_or(0)), next_key: None })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(StructVariantSerializer {
            variant: variant.to_string(),
            map: MapSerializer { entries: Vec::with_capacity(len), next_key: None },
        })
    }
}

struct SequenceSerializer {
    values: Vec<OrderedValue>,
}

impl ser::SerializeSeq for SequenceSerializer {
    type Ok = OrderedValue;
    type Error = OrderedValueError;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.values.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(OrderedValue::Array(self.values))
    }
}

impl ser::SerializeTuple for SequenceSerializer {
    type Ok = OrderedValue;
    type Error = OrderedValueError;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for SequenceSerializer {
    type Ok = OrderedValue;
    type Error = OrderedValueError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

struct MapSerializer {
    entries: Vec<(String, OrderedValue)>,
    next_key: Option<String>,
}

impl ser::SerializeMap for MapSerializer {
    type Ok = OrderedValue;
    type Error = OrderedValueError;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
        self.next_key = Some(key.serialize(StringKeySerializer)?);
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        let key = self
            .next_key
            .take()
            .ok_or_else(|| OrderedValueError("map value serialized before key".to_string()))?;
        self.entries.push((key, value.serialize(ValueSerializer)?));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        if self.next_key.is_some() {
            return Err(OrderedValueError("map key serialized without value".to_string()));
        }
        Ok(OrderedValue::Object(self.entries))
    }
}

impl ser::SerializeStruct for MapSerializer {
    type Ok = OrderedValue;
    type Error = OrderedValueError;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.entries.push((key.to_string(), value.serialize(ValueSerializer)?));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(OrderedValue::Object(self.entries))
    }
}

struct TupleVariantSerializer {
    variant: String,
    sequence: SequenceSerializer,
}

impl ser::SerializeTupleVariant for TupleVariantSerializer {
    type Ok = OrderedValue;
    type Error = OrderedValueError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        ser::SerializeSeq::serialize_element(&mut self.sequence, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(OrderedValue::Object(vec![(self.variant, OrderedValue::Array(self.sequence.values))]))
    }
}

struct StructVariantSerializer {
    variant: String,
    map: MapSerializer,
}

impl ser::SerializeStructVariant for StructVariantSerializer {
    type Ok = OrderedValue;
    type Error = OrderedValueError;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        ser::SerializeStruct::serialize_field(&mut self.map, key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(OrderedValue::Object(vec![(self.variant, OrderedValue::Object(self.map.entries))]))
    }
}

struct StringKeySerializer;

impl ser::Serializer for StringKeySerializer {
    type Ok = String;
    type Error = OrderedValueError;
    type SerializeSeq = ser::Impossible<String, OrderedValueError>;
    type SerializeTuple = ser::Impossible<String, OrderedValueError>;
    type SerializeTupleStruct = ser::Impossible<String, OrderedValueError>;
    type SerializeTupleVariant = ser::Impossible<String, OrderedValueError>;
    type SerializeMap = ser::Impossible<String, OrderedValueError>;
    type SerializeStruct = ser::Impossible<String, OrderedValueError>;
    type SerializeStructVariant = ser::Impossible<String, OrderedValueError>;

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        Ok(value.to_string())
    }

    fn serialize_char(self, value: char) -> Result<Self::Ok, Self::Error> {
        Ok(value.to_string())
    }

    fn serialize_bool(self, value: bool) -> Result<Self::Ok, Self::Error> {
        Ok(value.to_string())
    }

    fn serialize_i8(self, value: i8) -> Result<Self::Ok, Self::Error> {
        Ok(value.to_string())
    }
    fn serialize_i16(self, value: i16) -> Result<Self::Ok, Self::Error> {
        Ok(value.to_string())
    }
    fn serialize_i32(self, value: i32) -> Result<Self::Ok, Self::Error> {
        Ok(value.to_string())
    }
    fn serialize_i64(self, value: i64) -> Result<Self::Ok, Self::Error> {
        Ok(value.to_string())
    }
    fn serialize_u8(self, value: u8) -> Result<Self::Ok, Self::Error> {
        Ok(value.to_string())
    }
    fn serialize_u16(self, value: u16) -> Result<Self::Ok, Self::Error> {
        Ok(value.to_string())
    }
    fn serialize_u32(self, value: u32) -> Result<Self::Ok, Self::Error> {
        Ok(value.to_string())
    }
    fn serialize_u64(self, value: u64) -> Result<Self::Ok, Self::Error> {
        Ok(value.to_string())
    }

    fn serialize_f32(self, _value: f32) -> Result<Self::Ok, Self::Error> {
        invalid_map_key()
    }
    fn serialize_f64(self, _value: f64) -> Result<Self::Ok, Self::Error> {
        invalid_map_key()
    }
    fn serialize_bytes(self, _value: &[u8]) -> Result<Self::Ok, Self::Error> {
        invalid_map_key()
    }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        invalid_map_key()
    }
    fn serialize_some<T: ?Sized + Serialize>(self, _value: &T) -> Result<Self::Ok, Self::Error> {
        invalid_map_key()
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        invalid_map_key()
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        invalid_map_key()
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(variant.to_string())
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        invalid_map_key()
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        invalid_map_key()
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        invalid_map_key()
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        invalid_map_key()
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        invalid_map_key()
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        invalid_map_key()
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        invalid_map_key()
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        invalid_map_key()
    }
}

fn invalid_map_key<T>() -> Result<T, OrderedValueError> {
    Err(OrderedValueError("unsupported non-string map key".to_string()))
}

#[cfg(test)]
mod tests {
    use serde::Serialize;

    use super::{OrderedValue, to_ordered_value};

    #[derive(Serialize)]
    struct OrderedStruct {
        zeta: u32,
        alpha: bool,
    }

    #[test]
    fn preserves_struct_field_order() {
        let value = to_ordered_value(&OrderedStruct { zeta: 1, alpha: true }).unwrap();
        let OrderedValue::Object(entries) = value else { panic!("expected object") };
        assert_eq!(entries[0].0, "zeta");
        assert_eq!(entries[1].0, "alpha");
    }
}
