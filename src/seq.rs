use std::marker::PhantomData;

use serde::de::{DeserializeSeed, Error, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeSeq, Serializer};

///
/// Wraps an [`Iterator`] over serializable elements, and implements
/// [`Serialize`] by mapping the sequence of elements to the seq type
/// in the serde data model.
/// 
pub struct SerializableSeq<I>
    where I: Iterator + Clone
{
    data: I,
    len: Option<usize>
}

impl<I> SerializableSeq<I>
    where I: Iterator + Clone
{
    pub fn new(data: I) -> Self {
        Self { data: data, len: None }
    }

    pub fn new_with_len(data: I, len: usize) -> Self {
        assert!(data.size_hint().0 <= len);
        assert!(data.size_hint().1.is_none() || data.size_hint().1.unwrap() >= len);
        Self { data: data, len: Some(len) }
    }
}

impl<I> Serialize for SerializableSeq<I>
    where I: Iterator + Clone, 
        I::Item: Serialize
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut seq = serializer.serialize_seq(self.len)?;
        for x in self.data.clone() {
            seq.serialize_element(&x)?;
        }
        return seq.end();
    }
}

///
/// A [`DeserializeSeed`] that deserializes a sequence by deserializing each
/// element with a given [`DeserializeSeed`], and combining the result with a
/// given combinator.
/// 
/// # Length of the seed sequence
/// 
/// The iterator producing the seeds must contain at least one more seed than the
/// sequence to deserialize has elements. The reason is that for generic deserializers,
/// we don't know whether we reached the end unless we try to deserialize an element
/// beyond the end. However, to do that, we need a seed.
/// 
/// # Example
/// ```
/// # use feanor_serde::seq::*;
/// # use std::marker::PhantomData;
/// # use std::iter::repeat;
/// # use serde::de::DeserializeSeed;
/// let mut deserializer = serde_json::Deserializer::new(serde_json::de::StrRead::new("[1, 3, 5]"));
/// let deserialize_seed = DeserializeSeedSeq::new(
///     repeat(PhantomData::<i64>),
///     Vec::new(),
///     |mut current, next| { current.push(next); current }
/// );
/// assert_eq!(vec![1, 3, 5], deserialize_seed.deserialize(&mut deserializer).unwrap());
/// ```
/// 
pub struct DeserializeSeedSeq<'de, V, S, T, C>
    where V: Iterator<Item = S>,
        S: DeserializeSeed<'de>,
        C: FnMut(T, S::Value) -> T
{
    deserializer: PhantomData<&'de ()>,
    element_seed: PhantomData<S>,
    seeds: V,
    initial: T,
    collector: C
}

impl<'de, V, S, T, C> DeserializeSeedSeq<'de, V, S, T, C>
    where V: Iterator<Item = S>,
        S: DeserializeSeed<'de>,
        C: FnMut(T, S::Value) -> T
{
    pub fn new(seeds: V, initial: T, collector: C) -> Self {
        Self {
            deserializer: PhantomData,
            element_seed: PhantomData,
            seeds: seeds,
            initial: initial,
            collector: collector
        }
    }
}

impl<'de, V, S, T, C> DeserializeSeed<'de> for DeserializeSeedSeq<'de, V, S, T, C>
    where V: Iterator<Item = S>, 
        S: DeserializeSeed<'de>,
        C: FnMut(T, S::Value) -> T
{
    type Value = T;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where D: serde::Deserializer<'de>
    {
        struct ResultVisitor<'de, V, S, T, C>
            where V: Iterator<Item = S>,
                S: DeserializeSeed<'de>,
                C: FnMut(T, S::Value) -> T
        {
            deserializer: PhantomData<&'de ()>,
            element_seed: PhantomData<S>,
            seeds: V,
            initial: T,
            collector: C
        }

        impl<'de, V, S, T, C> Visitor<'de> for ResultVisitor<'de, V, S, T, C>
            where V: Iterator<Item = S>,
                S: DeserializeSeed<'de>,
                C: FnMut(T, S::Value) -> T
        {
            type Value = T;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a sequence of elements")
            }

            fn visit_seq<B>(mut self, mut seq: B) -> Result<Self::Value, B::Error>
                where B: SeqAccess<'de>
            {
                let mut result = self.initial;
                let mut current_len = 0;
                while let Some(seed) = self.seeds.next() {
                    let el = seq.next_element_seed(seed)?;
                    if let Some(el) = el {
                        current_len += 1;
                        result = (self.collector)(result, el);
                    } else {
                        return Ok(result);
                    }
                }
                return Err(Error::invalid_length(current_len, &format!("a sequence of length at most {}", current_len - 1).as_str()))
            }
        }

        return deserializer.deserialize_seq(ResultVisitor {
            deserializer: PhantomData,
            element_seed: PhantomData,
            collector: self.collector,
            initial: self.initial,
            seeds: self.seeds
        });
    }
}

#[cfg(test)]
use std::iter::{repeat, repeat_with};

#[cfg(test)]
fn testdata() -> Vec<Vec<i64>> {
    vec![
        Vec::new(),
        vec![1, 3],
        vec![1, 3, 4]
    ]
}

#[test]
fn test_serde_seq_postcard() {
    for data in testdata() {
        let serialized = postcard::to_allocvec(&SerializableSeq::new_with_len(data.iter().copied(), data.len())).unwrap();
        let result = DeserializeSeedSeq::new(repeat(PhantomData::<i64>), Vec::new(), |mut current, next| { current.push(next); current }).deserialize(
            &mut postcard::Deserializer::from_flavor(postcard::de_flavors::Slice::new(&serialized))
        ).unwrap();
        assert_eq!(data, result);
    }
}

#[test]
fn test_serde_seq_json() {
    for data in testdata() {
        let serialized = serde_json::to_string(&SerializableSeq::new(data.iter().copied())).unwrap();
        let result = DeserializeSeedSeq::new(repeat(PhantomData::<i64>), Vec::new(), |mut current, next| { current.push(next); current }).deserialize(
            &mut serde_json::Deserializer::from_str(&serialized)
        ).unwrap();
        assert_eq!(data, result);
        
        let serialized = serde_json::to_string(&SerializableSeq::new_with_len(data.iter().copied(), data.len())).unwrap();
        let result = DeserializeSeedSeq::new(repeat(PhantomData::<i64>), Vec::new(), |mut current, next| { current.push(next); current }).deserialize(
            &mut serde_json::Deserializer::from_str(&serialized)
        ).unwrap();
        assert_eq!(data, result);
    }
}

#[test]
fn test_deserialize_sequence_partially_json() {
    let data = vec![vec![1, 2, 3, 4], vec![1, 2, 3]];
    let serialized = serde_json::to_string(&SerializableSeq::new(data.iter().map(|data| SerializableSeq::new(data.iter())))).unwrap();
    let result = DeserializeSeedSeq::new(repeat_with(|| DeserializeSeedSeq::new(
            (0..5).map(|_| PhantomData::<i64>),
            Vec::new(),
            |mut current, next|  { current.push(next); current }
        )), 
        Vec::new(), 
        |mut current, next| { current.push(next); current }
    ).deserialize(
        &mut serde_json::Deserializer::from_str(&serialized)
    ).unwrap();
    assert_eq!(data, result);
}

#[test]
fn test_deserialize_sequence_partially_postcard() {
    let data = vec![vec![1, 2, 3, 4], vec![1, 2, 3]];
    let serialized = postcard::to_allocvec(&SerializableSeq::new_with_len(data.iter().map(|data| SerializableSeq::new_with_len(data.iter(), data.len())), data.len())).unwrap();
    let result = DeserializeSeedSeq::new(repeat_with(|| DeserializeSeedSeq::new(
            (0..5).map(|_| PhantomData::<i64>),
            Vec::new(),
            |mut current, next|  { current.push(next); current }
        )), 
        Vec::new(), 
        |mut current, next| { current.push(next); current }
    ).deserialize(
        &mut postcard::Deserializer::from_flavor(postcard::de_flavors::Slice::new(&serialized))
    ).unwrap();
    assert_eq!(data, result);
}