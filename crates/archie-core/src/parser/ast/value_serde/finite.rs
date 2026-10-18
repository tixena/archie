//! `Value::Float`: the `f64`, read back only when it is finite.

use core::borrow::Borrow;

use serde::de::Error as _;
use serde::{Deserialize as _, Deserializer, Serializer};

pub fn serialize<F, S>(float: F, serializer: S) -> Result<S::Ok, S::Error>
where
    F: Borrow<f64>,
    S: Serializer,
{
    serializer.serialize_f64(*float.borrow())
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let float = f64::deserialize(deserializer)?;
    if float.is_finite() {
        Ok(float)
    } else {
        Err(D::Error::custom("a float that is not finite"))
    }
}
