use std::fmt;

use serde::{
    de::{MapAccess, Visitor},
    Deserialize, Deserializer,
};

#[derive(Debug, PartialEq, Eq)]
pub struct OpaqueParameters(pub Vec<(String, String)>);

struct OpaqueParametersVisitor;

impl<'de> Visitor<'de> for OpaqueParametersVisitor {
    type Value = OpaqueParameters;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a map from string to string")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut vec = Vec::with_capacity(access.size_hint().unwrap_or(0));

        // While there are entries remaining in the input, add them
        // into our map.
        while let Some((key, value)) = access.next_entry()? {
            vec.push((key, value))
        }

        Ok(OpaqueParameters(vec))
    }
}

impl<'de> Deserialize<'de> for OpaqueParameters {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Instantiate our Visitor and ask the Deserializer to drive
        // it over the input data, resulting in an instance of MyMap.
        deserializer.deserialize_map(OpaqueParametersVisitor)
    }
}
