use std::marker::PhantomData;

use serde::de::{Error, DeserializeSeed, SeqAccess, Visitor};
use serde::Deserializer;

///
/// A [`DeserializeSeed`] that deserializes a tuple by deserializing its
/// first element with the given [`DeserializeSeed`], deriving another
/// [`DeserializeSeed`] from this first element, and using that to deserialize
/// the second element.
/// 
/// # Example
/// 
/// ```
/// # use feanor_serde::seq::*;
/// # use feanor_serde::dependent_tuple::*;
/// # use std::marker::PhantomData;
/// # use serde::de::DeserializeSeed;
/// let mut deserializer = serde_json::Deserializer::new(serde_json::de::StrRead::new("[3, [0, 0, 0]]"));
/// let deserialize_seed = DeserializeSeedDependentTuple::new(
///     PhantomData::<usize>,
///     |len| DeserializeSeedSeq::new(
///         (0..).map(|_| PhantomData::<i64>), 
///         Vec::with_capacity(len),
///         |mut current, next| { current.push(next); current }
///     )
/// );
/// assert_eq!(vec![0, 0, 0], deserialize_seed.deserialize(&mut deserializer).unwrap());
/// ```
/// 
pub struct DeserializeSeedDependentTuple<'de, T0, F, T1>
    where T0: DeserializeSeed<'de>,
        T1: DeserializeSeed<'de>,
        F: FnOnce(T0::Value) -> T1
{
    deserializer: PhantomData<&'de ()>,
    first: T0,
    derive_second: F
}

impl<'de, T0, F, T1> DeserializeSeedDependentTuple<'de, T0, F, T1>
    where T0: DeserializeSeed<'de>,
        T1: DeserializeSeed<'de>,
        F: FnOnce(T0::Value) -> T1
{
    pub fn new(first: T0, derive_second: F) -> Self {
        Self {
            deserializer: PhantomData,
            first: first,
            derive_second: derive_second
        }
    }
}

impl<'de, T0, F, T1> DeserializeSeed<'de> for DeserializeSeedDependentTuple<'de, T0, F, T1>
    where T0: DeserializeSeed<'de>,
        T1: DeserializeSeed<'de>,
        F: FnOnce(T0::Value) -> T1
{
    type Value = T1::Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where D: Deserializer<'de>
    {
        pub struct ResultVisitor<'de, T0, F, T1>
            where T0: DeserializeSeed<'de>,
                T1: DeserializeSeed<'de>,
                F: FnOnce(T0::Value) -> T1
        {
            deserializer: PhantomData<&'de ()>,
            first: T0,
            derive_second: F
        }

        impl<'de, T0, F, T1> Visitor<'de> for ResultVisitor<'de, T0, F, T1>
            where T0: DeserializeSeed<'de>,
                T1: DeserializeSeed<'de>,
                F: FnOnce(T0::Value) -> T1
        {
            type Value = T1::Value;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a tuple with 2 elements")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where A: SeqAccess<'de>
            {
                if let Some(first) = seq.next_element_seed(self.first)? {
                    if let Some(second) = seq.next_element_seed((self.derive_second)(first))? {
                        return Ok(second);
                    } else {
                        return Err(<A::Error as Error>::invalid_length(1, &"a tuple with 2 elements"));
                    }
                } else {
                    return Err(<A::Error as Error>::invalid_length(0, &"a tuple with 2 elements"));
                }
            }
        }

        return deserializer.deserialize_tuple(2, ResultVisitor {
            deserializer: PhantomData,
            first: self.first,
            derive_second: self.derive_second
        });
    }
}

#[cfg(test)]
use crate::seq::DeserializeSeedSeq;

#[test]
fn test_serde_postcard() {
    let data = (3, vec![0, 0, 0]);
    let serialized = postcard::to_allocvec(&data).unwrap();
    let result = DeserializeSeedDependentTuple::new(
        PhantomData::<usize>,
        |len| DeserializeSeedSeq::new(
            (0..len).map(|_| PhantomData::<i64>), 
            Vec::with_capacity(len),
            |mut current, next| { current.push(next); current }
        )
    ).deserialize(
        &mut postcard::Deserializer::from_flavor(postcard::de_flavors::Slice::new(&serialized))
    ).unwrap();
    assert_eq!(data.1, result);
}

#[test]
fn test_serde_json() {
    let data = (3, vec![0, 0, 0]);
    let serialized = serde_json::to_string(&data).unwrap();
    let result = DeserializeSeedDependentTuple::new(
        PhantomData::<usize>,
        |len| DeserializeSeedSeq::new(
            (0..(len + 1)).map(|_| PhantomData::<i64>), 
            Vec::with_capacity(len),
            |mut current, next| { current.push(next); current }
        )
    ).deserialize(
        &mut serde_json::Deserializer::from_str(&serialized)
    ).unwrap();
    assert_eq!(data.1, result);
}