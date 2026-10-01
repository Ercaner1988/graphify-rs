//! `Deger`: düğüm ve kenarların serbest ek alanlarının (`extra`) değer türü.
//!
//! Eskiden `serde_json::Value` idi; bu modeli JSON'a bağlıyordu ve ikili depoya
//! (rkyv) yazılamıyordu. `Deger` JSON'un tüm değerlerini taşır ve serde ile
//! `serde_json::Value` ile aynı biçimde yazılır/okunur: `graph.json` değişmez.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
#[serde(untagged)]
#[rkyv(serialize_bounds(
    __S: rkyv::ser::Writer + rkyv::ser::Allocator,
    __S::Error: rkyv::rancor::Source,
))]
#[rkyv(deserialize_bounds(__D::Error: rkyv::rancor::Source))]
#[rkyv(bytecheck(bounds(__C: rkyv::validation::ArchiveContext)))]
pub enum Deger {
    Null,
    Bool(bool),
    Tam(i64),
    /// i64'e sığmayan pozitif tamsayı (`serde_json::Value` da kayıpsız taşır).
    Pozitif(u64),
    Ondalik(f64),
    Metin(String),
    Liste(#[rkyv(omit_bounds)] Vec<Deger>),
    Sozluk(#[rkyv(omit_bounds)] BTreeMap<String, Deger>),
}

impl Deger {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Deger::Metin(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Deger>> {
        match self {
            Deger::Liste(v) => Some(v),
            _ => None,
        }
    }
}

impl From<&str> for Deger {
    fn from(s: &str) -> Self {
        Deger::Metin(s.to_string())
    }
}

impl From<String> for Deger {
    fn from(s: String) -> Self {
        Deger::Metin(s)
    }
}

impl From<bool> for Deger {
    fn from(b: bool) -> Self {
        Deger::Bool(b)
    }
}

impl<T: Into<Deger>> From<Vec<T>> for Deger {
    fn from(v: Vec<T>) -> Self {
        Deger::Liste(v.into_iter().map(Into::into).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// JSON gidiş-dönüşü `serde_json::Value` ile birebir: graph.json biçimi korunur.
    #[test]
    fn json_bicimi_value_ile_ayni() {
        let girdi = r#"{"a":null,"b":true,"c":-3,"d":2.5,"e":"İzmir","f":[1,"x"],"g":{"h":1.0},"i":18446744073709551615}"#;
        let d: BTreeMap<String, Deger> = serde_json::from_str(girdi).unwrap();
        assert_eq!(d["c"], Deger::Tam(-3));
        assert_eq!(
            d["g"],
            Deger::Sozluk(BTreeMap::from([("h".into(), Deger::Ondalik(1.0))]))
        );
        let v: serde_json::Value = serde_json::from_str(girdi).unwrap();
        assert_eq!(serde_json::to_value(&d).unwrap(), v);
    }

    #[test]
    fn ikili_gidis_donus() {
        let d = Deger::Liste(vec![
            Deger::Null,
            "ç".into(),
            Deger::Sozluk(BTreeMap::from([("k".into(), true.into())])),
        ]);
        let b = rkyv::to_bytes::<rkyv::rancor::Error>(&d).unwrap();
        assert_eq!(
            rkyv::from_bytes::<Deger, rkyv::rancor::Error>(&b).unwrap(),
            d
        );
    }
}
