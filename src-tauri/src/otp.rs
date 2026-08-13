use hmac::{Hmac, Mac};
use sha1::Sha1;
use sha2::{Sha256, Sha512};

use crate::model::Algorithm;

pub fn generate(secret: &[u8], algorithm: Algorithm, digits: u32, period: u64, now: u64) -> Result<String, String> {
    let message = (now / period).to_be_bytes();
    let digest = match algorithm {
        Algorithm::Sha1 => {
            let mut mac = Hmac::<Sha1>::new_from_slice(secret).map_err(|_| "HMAC 密钥无效")?;
            mac.update(&message); mac.finalize().into_bytes().to_vec()
        }
        Algorithm::Sha256 => {
            let mut mac = Hmac::<Sha256>::new_from_slice(secret).map_err(|_| "HMAC 密钥无效")?;
            mac.update(&message); mac.finalize().into_bytes().to_vec()
        }
        Algorithm::Sha512 => {
            let mut mac = Hmac::<Sha512>::new_from_slice(secret).map_err(|_| "HMAC 密钥无效")?;
            mac.update(&message); mac.finalize().into_bytes().to_vec()
        }
    };
    let offset = (digest[digest.len() - 1] & 0x0f) as usize;
    if offset + 4 > digest.len() { return Err("TOTP 摘要无效".into()); }
    let binary = ((digest[offset] as u32 & 0x7f) << 24) | ((digest[offset + 1] as u32) << 16) | ((digest[offset + 2] as u32) << 8) | digest[offset + 3] as u32;
    Ok(format!("{:0width$}", binary % 10_u32.pow(digits), width = digits as usize))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc_6238_sha1_vectors() {
        let secret = b"12345678901234567890";
        let cases = [(59, "94287082"), (1_111_111_109, "07081804"), (1_234_567_890, "89005924"), (2_000_000_000, "69279037")];
        for (timestamp, expected) in cases { assert_eq!(generate(secret, Algorithm::Sha1, 8, 30, timestamp).unwrap(), expected); }
    }

    #[test]
    fn rfc_6238_sha256_and_sha512_vectors() {
        let sha256 = b"12345678901234567890123456789012";
        let sha512 = b"1234567890123456789012345678901234567890123456789012345678901234";
        let cases = [
            (59, "46119246", "90693936"),
            (1_111_111_109, "68084774", "25091201"),
            (1_111_111_111, "67062674", "99943326"),
            (1_234_567_890, "91819424", "93441116"),
            (2_000_000_000, "90698825", "38618901"),
            (20_000_000_000, "77737706", "47863826"),
        ];
        for (timestamp, expected256, expected512) in cases {
            assert_eq!(generate(sha256, Algorithm::Sha256, 8, 30, timestamp).unwrap(), expected256);
            assert_eq!(generate(sha512, Algorithm::Sha512, 8, 30, timestamp).unwrap(), expected512);
        }
    }
}
