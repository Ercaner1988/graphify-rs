//! graphify'ın kendi yazıp okuduğu dosyaların ikili biçimi (rkyv).
//!
//! Dosya = 4 bayt imza + 4 bayt biçim sürümü (LE) + rkyv yükü. Sürüm ya da imza
//! tutmayan dosya `None` döner: çağıran onu yok sayıp yeniden üretir (önbellek,
//! değişim dizini) ya da JSON'a düşer (graf). Bozuk yük rkyv'nin doğrulamasından
//! (bytecheck) geçemez; güvensiz okuma yok.

use rkyv::api::high::{HighDeserializer, HighSerializer, HighValidator};
use rkyv::bytecheck::CheckBytes;
use rkyv::rancor::Error;
use rkyv::ser::allocator::ArenaHandle;
use rkyv::util::AlignedVec;

const IMZA: &[u8; 4] = b"GRFY";
/// Model değişince artırılır; eski dosyalar sessizce yeniden üretilir.
pub const SURUM: u32 = 1;

pub fn kodla<T>(deger: &T) -> Result<Vec<u8>, Error>
where
    T: for<'a> rkyv::Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, Error>>,
{
    let yuk = rkyv::to_bytes::<Error>(deger)?;
    let mut out = Vec::with_capacity(8 + yuk.len());
    out.extend_from_slice(IMZA);
    out.extend_from_slice(&SURUM.to_le_bytes());
    out.extend_from_slice(&yuk);
    Ok(out)
}

pub fn coz<T>(bayt: &[u8]) -> Option<T>
where
    T: rkyv::Archive,
    T::Archived: for<'a> CheckBytes<HighValidator<'a, Error>>
        + rkyv::Deserialize<T, HighDeserializer<Error>>,
{
    let (bas, yuk) = bayt.split_at_checked(8)?;
    if &bas[..4] != IMZA || bas[4..] != SURUM.to_le_bytes() {
        return None;
    }
    // Dosyadan okunan Vec hizalı değil; rkyv hizalı tampon ister.
    let mut hizali = AlignedVec::<16>::with_capacity(yuk.len());
    hizali.extend_from_slice(yuk);
    rkyv::from_bytes::<T, Error>(&hizali).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imza_ve_surum_denetlenir() {
        let b = kodla(&vec![1u32, 2, 3]).unwrap();
        assert_eq!(coz::<Vec<u32>>(&b), Some(vec![1, 2, 3]));
        let mut eski = b.clone();
        eski[4] = 0;
        assert_eq!(coz::<Vec<u32>>(&eski), None, "eski sürüm yok sayılmalı");
        assert_eq!(coz::<Vec<u32>>(b"{\"json\": 1}"), None);
        assert_eq!(coz::<Vec<u32>>(&b[..5]), None);
        let mut bozuk = b;
        let n = bozuk.len();
        bozuk[n - 1] = 0xFF;
        assert_eq!(
            coz::<Vec<u32>>(&bozuk),
            None,
            "doğrulanmayan yük reddedilmeli"
        );
    }
}
