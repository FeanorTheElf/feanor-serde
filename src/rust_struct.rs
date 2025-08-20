
///
/// Macro to implement [`serde::de::DeserializeSeed`] for a custom type.
/// 
/// More concretely, when using this macro, you will define a struct and
/// a [`serde::de::DeserializeSeed`] for each of its fields (which can access data from
/// your custom type). The custom type can then be used to deserialize the
/// struct, by deserializing each of its fields separately with the derived
/// [`serde::de::DeserializeSeed`]s.
/// 
/// # Example
/// 
/// As a very simple example, this macro can be used as a poor man's version
/// of `#[derive(Deserialize)]` as follows.
/// 
/// The function `deserializer()` in
/// ```rust
/// # use feanor_serde::impl_deserialize_seed_for_dependent_struct;
/// # use serde::*;
/// # use std::marker::PhantomData;
/// struct FooDeserializeSeed;
/// impl_deserialize_seed_for_dependent_struct!{
///     pub struct Foo<'de> using FooDeserializeSeed {
///         a: i64: |_| PhantomData::<i64>,
///         b: String: |_| PhantomData::<String>
///     }
/// }
/// fn deserializer<'de>() -> impl serde::de::DeserializeSeed<'de, Value = Foo<'de>> {
///     FooDeserializeSeed
/// }
/// ```
/// is roughly equivalent to `deserializer()` as in
/// ```rust
/// # use serde::*;
/// # use std::marker::PhantomData;
/// #[derive(Deserialize)]
/// struct Foo {
///     a: i64,
///     b: String
/// }
/// fn deserializer<'de>() -> impl serde::de::DeserializeSeed<'de, Value = Foo> {
///     PhantomData::<Foo>
/// }
/// ```
/// 
/// It becomes more interesting if fields of the result struct should be deserialized
/// using a [`serde::de::DeserializeSeed`], since in this case, it cannot be achieved using `#[derive(Deserialize)]`
/// anymore. Note however that [`crate::impl_deserialize_seed_for_dependent_struct!`] can only implement
/// [`serde::de::DeserializeSeed`] for a type in terms of more basic [`serde::de::DeserializeSeed`]s. Hence, the leaves of the
/// "deserialization-tree" must still be implemented manually (this is also the case for `#[derive(Deserialize)]`
/// of course, but the leaves here are usually std type `i64`, `&[u8]` or `String`, for which the implementation
/// of [`serde::Deserialize`] is contained in `serde`).
/// ```rust
/// # use feanor_serde::impl_deserialize_seed_for_dependent_struct;
/// # use serde::*;
/// # use serde::de::DeserializeSeed;
/// # use std::marker::PhantomData;
/// #[derive(Copy, Clone)]
/// struct LeafDeserializeSeed {
///     mask_with: i64
/// }
/// impl<'de> DeserializeSeed<'de> for LeafDeserializeSeed {
///     type Value = i64;
/// 
///     fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
///         where D: serde::Deserializer<'de> 
///     {
///         Ok(self.mask_with ^ i64::deserialize(deserializer)?)
///     }
/// }
/// 
/// struct FooDeserializeSeed {
///     deserialize_a: LeafDeserializeSeed,
///     deserialize_b: LeafDeserializeSeed
/// };
/// 
/// impl_deserialize_seed_for_dependent_struct!{
///     pub struct Foo<'de> using FooDeserializeSeed {
///         a: i64: |seed: &FooDeserializeSeed| seed.deserialize_a,
///         b: i64: |seed: &FooDeserializeSeed| seed.deserialize_b
///     }
/// }
/// 
/// let mut deserializer = serde_json::Deserializer::new(serde_json::de::StrRead::new(r#"{
///     "a": 1,
///     "b": 0
/// }"#));
/// let deserialize_seed = FooDeserializeSeed {
///     deserialize_a: LeafDeserializeSeed { mask_with: 0 },
///     deserialize_b: LeafDeserializeSeed { mask_with: 1 }
/// };
/// let foo = deserialize_seed.deserialize(&mut deserializer).unwrap();
/// assert_eq!(1, foo.a);
/// assert_eq!(0 ^ 1, foo.b); // `b` should have been masked with `1` during deserialization
/// ```
/// 
/// Note that if `FooDeserializeSeed` should have generic parameters, these should be passed
/// in the following way:
/// ```rust
/// # use feanor_serde::impl_deserialize_seed_for_dependent_struct;
/// # use serde::*;
/// # use serde::de::DeserializeSeed;
/// # use std::marker::PhantomData;
/// struct FooDeserializeSeed<S>(S);
/// 
/// impl_deserialize_seed_for_dependent_struct!{
///     <{'de, S}> pub struct Foo<{'de, S}> using FooDeserializeSeed<S> {
///         a: S::Value: |seed: &FooDeserializeSeed<S>| seed.0.clone()
///     } where S: DeserializeSeed<'de> + Clone
/// }
/// ```
/// 
/// # But the lifetimes aren't exactly what they should be!?
/// 
/// Well, it depends on what you are trying to express. I implemented what I consider
/// the be the most powerful option, namely to allow `Foo` to borrow data from the
/// [`serde::Deserializer`], and thus depend on `'de`.
/// 
/// In the simpler (and possibly more common) case that `Foo` should own its data and
/// outlive the [`serde::Deserializer`], this causes a problem:
/// ```compile_fail
/// # use feanor_serde::impl_deserialize_seed_for_dependent_struct;
/// # use serde::*;
/// # use serde::de::DeserializeSeed;
/// # use std::marker::PhantomData;
/// struct FooDeserializeSeed;
/// 
/// impl_deserialize_seed_for_dependent_struct!{
///     pub struct Foo<'de> using FooDeserializeSeed {
///         a: String: |_| PhantomData::<String>
///     }
/// }
/// 
/// // compile error: `json_str` would have to have lifetime 'foo_lifetime
/// fn deserialize_foo_from_json<'foo_lifetime>(json_str: &str) -> Foo<'foo_lifetime> {
///     let mut deserializer = serde_json::Deserializer::new(serde_json::de::StrRead::new(json_str));
///     return FooDeserializeSeed.deserialize(&mut deserializer).unwrap();
/// }
/// ```
/// However, in these cases, it should suffice to manually convert `Foo` into some self-defined
/// struct `FooOwned` before returning it.
/// ```rust
/// # use feanor_serde::impl_deserialize_seed_for_dependent_struct;
/// # use serde::*;
/// # use serde::de::DeserializeSeed;
/// # use std::marker::PhantomData;
/// # struct FooDeserializeSeed;
/// # impl_deserialize_seed_for_dependent_struct!{
/// #     pub struct Foo<'de> using FooDeserializeSeed {
/// #         a: String: |_| PhantomData::<String>
/// #     }
/// # }
/// struct FooOwned {
///     a: String
/// }
/// fn deserialize_foo_from_json(json_str: &str) -> FooOwned {
///     let mut deserializer = serde_json::Deserializer::new(serde_json::de::StrRead::new(json_str));
///     let foo_borrowed = FooDeserializeSeed.deserialize(&mut deserializer).unwrap();
///     return FooOwned { a: foo_borrowed.a };
/// }
/// ```
/// 
#[macro_export]
macro_rules! impl_deserialize_seed_for_dependent_struct {
    (
        pub struct $deserialize_result_struct_name:ident<'de> using $deserialize_seed_type:ty {
            $($field:ident: $type:ty: $local_deserialize_seed:expr),*
        }
    ) => {
        impl_deserialize_seed_for_dependent_struct!{ <{'de,}> pub struct $deserialize_result_struct_name<{'de,}> using $deserialize_seed_type {
            $($field: $type: $local_deserialize_seed),*
        } where }
    };
    (
        <{'de, $($gen_args:tt)*}> pub struct $deserialize_result_struct_name:ident<{'de, $($deserialize_result_gen_args:tt)*}> using $deserialize_seed_type:ty {
            $($field:ident: $type:ty: $local_deserialize_seed:expr),*
        } where $($constraints:tt)*
    ) => {
        pub struct $deserialize_result_struct_name<'de, $($deserialize_result_gen_args)*> 
            where $($constraints)*
        {
            deserializer: std::marker::PhantomData<&'de ()>,
            $(pub $field: $type),*
        }
        impl<'de, $($gen_args)*> serde::de::DeserializeSeed<'de> for $deserialize_seed_type
            where $($constraints)*
        {
            type Value = $deserialize_result_struct_name<'de, $($deserialize_result_gen_args)*>;

            fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                where D: serde::Deserializer<'de> 
            {
                use serde::de::*;

                type Field = Option<u32>;

                const fn get_const_len<const N: usize>(_: [&'static str; N]) -> usize {
                    N
                }
                const FIELD_COUNT: usize = get_const_len([$(stringify!($field)),*]);

                struct FieldVisitor;
                impl<'de> Visitor<'de> for FieldVisitor {

                    type Value = Field;

                    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        std::fmt::Formatter::write_str(f, "field identifier")
                    }

                    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
                        where E: Error
                    {
                        if value >= FIELD_COUNT as u64 {
                            Ok(None)
                        } else {
                            Ok(Some(value as u32))
                        }
                    }

                    #[allow(unused_assignments)]
                    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                        where E: Error
                    {
                        let mut current = 0;
                        $(
                            if value == stringify!($field) {
                                return Ok(Some(current));
                            }
                            current += 1;
                        )*
                        return Ok(None);
                    }

                    #[allow(unused_assignments)]
                    fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
                        where E: Error
                    {
                        let mut current = 0;
                        $(
                            if value == stringify!($field).as_bytes() {
                                return Ok(Some(current));
                            }
                            current += 1;
                        )*
                        return Ok(None);
                    }
                }

                struct FieldDeserializer;
                impl<'de> DeserializeSeed<'de> for FieldDeserializer {
                    type Value = Field;

                    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                        where D: serde::Deserializer<'de> 
                    {
                        deserializer.deserialize_identifier(FieldVisitor)
                    }
                }

                struct ResultVisitor<'de, $($gen_args)*>
                    where $($constraints)*
                {
                    deserializer: std::marker::PhantomData<&'de ()>,
                    deserialize_seed_base: $deserialize_seed_type
                }

                impl<'de, $($gen_args)*> Visitor<'de> for ResultVisitor<'de, $($gen_args)*>
                    where $($constraints)*
                {
                    type Value = $deserialize_result_struct_name<'de, $($deserialize_result_gen_args)*>;

                    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        std::fmt::Formatter::write_str(f, concat!("struct ", stringify!($deserialize_result_struct_name)))
                    }

                    #[allow(unused_assignments)]
                    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                        where A: SeqAccess<'de>
                    {
                        let mut encountered_fields = 0;
                        Ok($deserialize_result_struct_name {
                            deserializer: std::marker::PhantomData,
                            $($field: {
                                let current_deserialize_seed = ($local_deserialize_seed)(&self.deserialize_seed_base);
                                let field_value = match seq.next_element_seed(current_deserialize_seed)? {
                                    Some(value) => value,
                                    None => return Err(Error::invalid_length(encountered_fields, &format!("struct {} with {} elements", stringify!($deserialize_result_struct_name), FIELD_COUNT).as_str()))
                                };
                                encountered_fields += 1;
                                field_value
                            }),*
                        })
                    }

                    #[allow(unused_assignments)]
                    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
                        where M: MapAccess<'de>
                    {
                        $(
                            let mut $field: Option<$type> = None;
                        )*
                        while let Some(key) = map.next_key_seed(FieldDeserializer)? {
                            if let Some(key) = key {
                                let mut current = 0;
                                $(
                                    if key == current {
                                        if $field.is_some() {
                                            return Err(<M::Error as Error>::duplicate_field(stringify!($field)));
                                        }
                                        let current_deserialize_seed = ($local_deserialize_seed)(&self.deserialize_seed_base);
                                        $field = Some(map.next_value_seed(current_deserialize_seed)?);
                                    }
                                    current += 1;
                                )*
                            }
                        }
                        $(
                            let $field: $type = match $field {
                                None => return Err(<M::Error as Error>::missing_field(stringify!($field))),
                                Some(value) => value
                            };
                        )*
                        return Ok($deserialize_result_struct_name { 
                            deserializer: std::marker::PhantomData,
                            $($field),*
                        });
                    }
                }

                return deserializer.deserialize_struct(
                    stringify!($deserialize_result_struct_name),
                    &[$(stringify!($field)),*],
                    ResultVisitor { deserialize_seed_base: self, deserializer: std::marker::PhantomData }
                )
            }
        }
    };
}

#[cfg(test)]
use serde::Serialize;
#[cfg(test)]
use serde::de::DeserializeSeed;

#[test]
fn test_serde_seq_postcard() {

    #[derive(Serialize)]
    #[serde(rename = "Foo")]
    struct SerializableFoo {
        a: i64,
        b: String
    }

    struct DeserializeSeedFoo;

    impl_deserialize_seed_for_dependent_struct! {
        pub struct Foo<'de> using DeserializeSeedFoo {
            a: i64: |_| std::marker::PhantomData,
            b: String: |_| std::marker::PhantomData
        }
    }

    let serialized = postcard::to_allocvec(&SerializableFoo { a: 42, b: "the answer".to_owned() }).unwrap();
    let result = DeserializeSeedFoo.deserialize(
        &mut postcard::Deserializer::from_flavor(postcard::de_flavors::Slice::new(&serialized))
    ).unwrap();
    assert_eq!(42, result.a);
    assert_eq!("the answer", result.b);
}

#[test]
fn test_serde_seq_json() {
    #[derive(Serialize)]
    #[serde(rename = "Foo")]
    struct SerializableFoo {
        a: i64,
        b: String
    }

    struct DeserializeSeedFoo;

    impl_deserialize_seed_for_dependent_struct! {
        pub struct Foo<'de> using DeserializeSeedFoo {
            a: i64: |_| std::marker::PhantomData,
            b: String: |_| std::marker::PhantomData
        }
    }

    let serialized = serde_json::to_string(&SerializableFoo { a: 42, b: "the answer".to_owned() }).unwrap();
    let result = DeserializeSeedFoo.deserialize(
        &mut serde_json::Deserializer::from_str(&serialized)
    ).unwrap();
    assert_eq!(42, result.a);
    assert_eq!("the answer", result.b);
}