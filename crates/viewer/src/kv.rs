//! Valve KV1 helpers

use std::{collections::HashMap, fmt::Debug, ops::Deref};

use glam::Vec3;
use serde::{Deserialize, Deserializer, de::DeserializeOwned};

pub fn deserialize_kv_case_insensitive<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: DeserializeOwned,
    D: Deserializer<'de>,
{
    use vdf_reader::entry::Entry;

    use std::collections::BTreeMap as Map;

    let map = Map::<String, Entry>::deserialize(deserializer)?;
    let lower: HashMap<String, Entry> = map
        .into_iter()
        .map(|(k, v)| (k.to_lowercase(), json_value_lowercase_keys(v)))
        .collect();
    T::deserialize(Entry::Table(lower.into())).map_err(serde::de::Error::custom)
}

fn json_value_lowercase_keys(v: vdf_reader::entry::Entry) -> vdf_reader::entry::Entry {
    use vdf_reader::entry::Entry;
    match v {
        Entry::Table(map) => Entry::Table(
            map.into_iter()
                .map(|(k, v)| (k.to_lowercase(), json_value_lowercase_keys(v)))
                .collect::<HashMap<String, Entry>>()
                .into(),
        ),
        _ => v,
    }
}

pub type QAngle = Vector;

#[derive(Clone, Copy, PartialEq)]
pub struct Vector(pub Vec3);

impl<'de> serde::Deserialize<'de> for Vector {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct VectorVisitor;

        impl<'de> serde::de::Visitor<'de> for VectorVisitor {
            type Value = Vector;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(
                    "a string with 3 floating point values, each component separated by a space",
                )
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let mut s = v.splitn(3, ' ');
                let x = s.next().ok_or(serde::de::Error::invalid_length(0, &self))?;
                let y = s.next().ok_or(serde::de::Error::invalid_length(1, &self))?;
                let z = s.next().ok_or(serde::de::Error::invalid_length(2, &self))?;
                let x = x.parse().map_err(serde::de::Error::custom)?;
                let y = y.parse().map_err(serde::de::Error::custom)?;
                let z = z.parse().map_err(serde::de::Error::custom)?;
                Ok(Vector(Vec3::new(x, y, z)))
            }
        }

        deserializer.deserialize_str(VectorVisitor)
    }
}

impl Debug for Vector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(stringify!(Vector))
            .field(&self.x)
            .field(&self.y)
            .field(&self.z)
            .finish()
    }
}

impl Deref for Vector {
    type Target = Vec3;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Vec3> for Vector {
    fn from(value: Vec3) -> Self {
        Self(value)
    }
}

impl From<Vector> for Vec3 {
    fn from(value: Vector) -> Self {
        value.0
    }
}
