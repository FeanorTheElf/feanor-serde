# feanor-serde

This is a tiny library that provides some features to work with [serde](https://serde.rs/), in particular [`serde::de::DeserializeSeed`]s.
I require these, since [feanor-math](https://github.com/FeanorTheElf/feanor-math) and libraries building on it often need to serialize types that only exist as associated to some master object - e.g. ring elements, which belong to a ring.
Hence, these can only be serialized and deserialized when given access to the master object, and thus must use [`serde::de::DeserializeSeed`].
Unfortunately, while serde makes it very convenient to implement [`serde::Deserialize`], not much utilities are provided for [`serde::de::DeserializeSeed`].
This library is a very small set of such utilities.