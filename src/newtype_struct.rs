use std::marker::PhantomData;

use serde::de::{DeserializeSeed, Visitor};
use serde::ser::{Serialize, Serializer};
use serde::Deserializer;

///
/// Wraps an serializable object, and implements [`Serialize`] by mapping
/// to the newtype_struct type in the serde data model, using the wrapped object
/// as the content of the newtype struct.
/// 
pub struct SerializableNewtypeStruct<T>
    where T: Serialize
{
    name: &'static str,
    data: T 
}

impl<T> SerializableNewtypeStruct<T>
    where T: Serialize
{
    pub fn new(name: &'static str, data: T) -> Self {
        Self { name, data }
    }
}

impl<T> Serialize for SerializableNewtypeStruct<T>
    where T: Serialize
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        serializer.serialize_newtype_struct(self.name, &self.data)
    }
}

///
/// A [`DeserializeSeed`] that deserializes a newtype setruct by deserializing
/// the content with the given [`DeserializeSeed`].
/// 
/// # Example
/// ```
/// # use feanor_serde::newtype_struct::*;
/// # use std::marker::PhantomData;
/// # use serde::de::DeserializeSeed;
/// let mut deserializer = serde_json::Deserializer::new(serde_json::de::StrRead::new("1"));
/// let deserialize_seed = DeserializeSeedNewtypeStruct::new("Foo", PhantomData::<i64>);
/// assert_eq!(1, deserialize_seed.deserialize(&mut deserializer).unwrap());
/// ```
/// 
pub struct DeserializeSeedNewtypeStruct<'de, S>
    where S: DeserializeSeed<'de>
{
    deserializer: PhantomData<&'de ()>,
    name: &'static str,
    seed: S
}

impl<'de, S> DeserializeSeedNewtypeStruct<'de, S>
    where S: DeserializeSeed<'de>
{
    pub fn new(name: &'static str, seed: S) -> Self {
        Self { deserializer: PhantomData, name, seed }
    }
}

impl<'de, S> DeserializeSeed<'de> for DeserializeSeedNewtypeStruct<'de, S>
    where S: DeserializeSeed<'de>
{
    type Value = S::Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where D: Deserializer<'de>
    {
        struct NewtypeStructVisitor<'de, S: DeserializeSeed<'de>> {
            seed: S,
            name: &'static str,
            deserializer: PhantomData<&'de ()>
        }
    
        impl<'de, S: DeserializeSeed<'de>> Visitor<'de> for NewtypeStructVisitor<'de, S> {
            type Value = S::Value;
    
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a newtype struct named {}", self.name)
            }
    
            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                where D: Deserializer<'de>
            {
                self.seed.deserialize(deserializer)
            }
        }
    
        return deserializer.deserialize_newtype_struct(self.name, NewtypeStructVisitor { seed: self.seed, name: self.name, deserializer: PhantomData });
    }
}

#[cfg(test)]
fn testdata() -> Vec<(&'static str, &'static str, i64)> {
    vec![
        ("Foo", "Bar", 1),
        ("Foo", "foo", 2),
        ("Bar", "Foo", 1),
        ("", "foo", 3),
    ]
}

#[test]
fn test_serde_seq_postcard() {
    for (name, notname, payload) in testdata() {
        let serialized = postcard::to_allocvec(&SerializableNewtypeStruct::new(name, payload)).unwrap();
        let result = DeserializeSeedNewtypeStruct::new(name, PhantomData::<i64>).deserialize(
            &mut postcard::Deserializer::from_flavor(postcard::de_flavors::Slice::new(&serialized))
        ).unwrap();
        assert_eq!(result, payload);

        let serialized = postcard::to_allocvec(&SerializableNewtypeStruct::new(name, payload)).unwrap();
        let result = DeserializeSeedNewtypeStruct::new(notname, PhantomData::<i64>).deserialize(
            &mut postcard::Deserializer::from_flavor(postcard::de_flavors::Slice::new(&serialized))
        );
        assert!(result.is_err() || result.unwrap() == payload);
    }
}

#[test]
fn test_serde_seq_json() {
    for (name, notname, payload) in testdata() {
        let serialized = serde_json::to_string(&SerializableNewtypeStruct::new(name, payload)).unwrap();
        let result = DeserializeSeedNewtypeStruct::new(name, PhantomData::<i64>).deserialize(
            &mut serde_json::Deserializer::from_str(&serialized)
        ).unwrap();
        assert_eq!(result, payload);
        
        let serialized = serde_json::to_string(&SerializableNewtypeStruct::new(name, payload)).unwrap();
        let result = DeserializeSeedNewtypeStruct::new(notname, PhantomData::<i64>).deserialize(
            &mut serde_json::Deserializer::from_str(&serialized)
        );
        assert!(result.is_err() || result.unwrap() == payload);
    }
}