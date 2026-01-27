use serde::{
    Deserialize, Serialize,
    de::{self, Visitor},
};
use strum::{Display, EnumString};

mod ream;

pub trait Client {}

#[derive(Debug, Clone, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum ClientKind {
    Ream,
    Zeam,
    Qlean,
    Lantern,
    Lighthouse,
    Grandine,
    Ethrex,
}

impl Serialize for ClientKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ClientKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Vis;

        impl<'de> Visitor<'de> for Vis {
            type Value = ClientKind;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a valid lean client identifier")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                v.parse().map_err(de::Error::custom)
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(&v)
            }
        }

        deserializer.deserialize_str(Vis)
    }
}
