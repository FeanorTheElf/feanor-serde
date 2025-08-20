
///
/// Same as [`crate::impl_deserialize_seed_for_dependent_struct!`] but for enums.
/// 
/// The syntax is as follows:
/// ```
/// # use feanor_serde::*;
/// # use serde::de::DeserializeSeed;
/// # use std::marker::PhantomData;
/// struct DeserializeSeedEither<S1, S2>
///     where S1: for<'de> DeserializeSeed<'de>,
///         S2: for<'de> DeserializeSeed<'de>
/// {
///     seed1: S1,
///     seed2: S2
/// }
/// impl_deserialize_seed_for_dependent_enum! {
///     <{'de, S1, S2}> pub enum Either<{'de, S1, S2}> using DeserializeSeedEither<S1, S2> {
///         First(<S1 as DeserializeSeed<'de>>::Value): |seed: DeserializeSeedEither<S1, S2>| seed.seed1,
///         Second(<S2 as DeserializeSeed<'de>>::Value): |seed: DeserializeSeedEither<S1, S2>| seed.seed2
///     }
///         where S1: for<'de2> DeserializeSeed<'de2>,
///             S2: for<'de2> DeserializeSeed<'de2>
/// }

/// 
/// let mut deserializer = serde_json::Deserializer::new(serde_json::de::StrRead::new(r#"{
///     "First": 1
/// }"#));
/// let deserialize_seed = DeserializeSeedEither {
///     seed1: PhantomData::<i64>,
///     seed2: PhantomData::<String>
/// };
/// let result = deserialize_seed.deserialize(&mut deserializer).unwrap();
/// match result {
///     Either::First(x) => assert_eq!(1, x.0),
///     _ => unreachable!()
/// }
/// ```
/// 
#[macro_export]
macro_rules! impl_deserialize_seed_for_dependent_enum {
    (
        pub enum $deserialize_result_enum_name:ident<'de> using $deserialize_seed_type:ty {
            $($variant:ident($type:ty): $local_deserialize_seed:expr),*
        }
    ) => {
        impl_deserialize_seed_for_dependent_enum!{ <{'de,}> pub enum $deserialize_result_enum_name<{'de,}> using $deserialize_seed_type {
            $($variant($type): $local_deserialize_seed),*
        } where }
    };
    (
        <{'de, $($gen_args:tt)*}> pub enum $deserialize_result_enum_name:ident<{'de, $($deserialize_result_gen_args:tt)*}> using $deserialize_seed_type:ty {
            $($variant:ident($type:ty): $local_deserialize_seed:expr),*
        } where $($constraints:tt)*
    ) => {
        #[allow(dead_code)]
        pub enum $deserialize_result_enum_name<'de, $($deserialize_result_gen_args)*> 
            where $($constraints)*
        {
            $($variant(($type, std::marker::PhantomData<&'de ()>))),*
        }
        impl<'de, $($gen_args)*> serde::de::DeserializeSeed<'de> for $deserialize_seed_type
            where $($constraints)*
        {
            type Value = $deserialize_result_enum_name<'de, $($deserialize_result_gen_args)*>;

            fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                where D: serde::Deserializer<'de> 
            {
                use serde::de::*;

                type Field = u32;

                const fn get_const_len<const N: usize>(_: [&'static str; N]) -> usize {
                    N
                }
                const FIELDS: &[&'static str] = &[$(stringify!($variant)),*];
                const FIELD_COUNT: usize = get_const_len([$(stringify!($variant)),*]);

                struct FieldVisitor;
                impl<'de> Visitor<'de> for FieldVisitor {

                    type Value = Field;

                    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        std::fmt::Formatter::write_str(f, "variant identifier")
                    }

                    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
                        where E: Error
                    {
                        if value >= FIELD_COUNT as u64 {Err(serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(value), &format!("variant index should be < {}", FIELD_COUNT).as_str()))
                        } else {
                            Ok(value as u32)
                        }
                    }

                    #[allow(unused_assignments)]
                    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                        where E: Error
                    {
                        let mut current = 0;
                        $(
                            if value == stringify!($variant) {
                                return Ok(current);
                            }
                            current += 1;
                        )*
                        return Err(serde::de::Error::unknown_variant(value, FIELDS));
                    }

                    #[allow(unused_assignments)]
                    fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
                        where E: Error
                    {
                        let mut current = 0;
                        $(
                            if value == stringify!($variant).as_bytes() {
                                return Ok(current);
                            }
                            current += 1;
                        )*
                        let value = &String::from_utf8_lossy(value);
                        return Err(serde::de::Error::unknown_variant(value, FIELDS));
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
                    type Value = $deserialize_result_enum_name<'de, $($deserialize_result_gen_args)*>;

                    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        std::fmt::Formatter::write_str(f, concat!("enum ", stringify!($deserialize_result_enum_name)))
                    }

                    #[allow(unused_assignments)]
                    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
                        where A: serde::de::EnumAccess<'de>
                    {
                        let variant = serde::de::EnumAccess::variant_seed(data, FieldDeserializer)?;
                        let mut current = 0;
                        $(
                            if variant.0 == current {
                                return Ok($deserialize_result_enum_name::$variant((
                                    serde::de::VariantAccess::newtype_variant_seed(variant.1, ($local_deserialize_seed)(self.deserialize_seed_base))?,
                                    std::marker::PhantomData
                                )));
                            }
                            current += 1;
                        )*
                        unreachable!()
                    }
                }

                return deserializer.deserialize_enum(
                    stringify!($deserialize_result_enum_name),
                    &[$(stringify!($variant)),*],
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
    #[allow(dead_code)]
    enum SerializableFoo {
        A(i64), B(String)
    }

    struct DeserializeSeedFoo;

    impl_deserialize_seed_for_dependent_enum! {
        pub enum Foo<'de> using DeserializeSeedFoo {
            A(i64): |_| std::marker::PhantomData,
            B(String): |_| std::marker::PhantomData
        }
    }

    let serialized = postcard::to_allocvec(&SerializableFoo::B("the answer".to_owned())).unwrap();
    let result = DeserializeSeedFoo.deserialize(
        &mut postcard::Deserializer::from_flavor(postcard::de_flavors::Slice::new(&serialized))
    ).unwrap();
    match result {
        Foo::B(m) => assert_eq!("the answer", m.0),
        _ => unreachable!()
    }
}

#[test]
fn test_serde_seq_json() {
    #[derive(Serialize)]
    #[serde(rename = "Foo")]
    #[allow(dead_code)]
    enum SerializableFoo {
        A(i64), B(String)
    }

    struct DeserializeSeedFoo;

    impl_deserialize_seed_for_dependent_enum! {
        pub enum Foo<'de> using DeserializeSeedFoo {
            A(i64): |_| std::marker::PhantomData,
            B(String): |_| std::marker::PhantomData
        }
    }

    let serialized = serde_json::to_string(&SerializableFoo::B("the answer".to_owned())).unwrap();
    let result = DeserializeSeedFoo.deserialize(
        &mut serde_json::Deserializer::from_str(&serialized)
    ).unwrap();
    match result {
        Foo::B(m) => assert_eq!("the answer", m.0),
        _ => unreachable!()
    }
}