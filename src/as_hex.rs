use hex;

use serde::{Serializer, Deserializer, Deserialize, de};

pub fn serialize<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer, T: AsRef<[u8]>
{
    let string = String::from("0x") + hex::encode(value).as_str();
    serializer.collect_str(&string)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where D: Deserializer<'de>
{
    hex::decode(&String::deserialize(deserializer)?.as_str()[2..]).map_err(de::Error::custom)
}

