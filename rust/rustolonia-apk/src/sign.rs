//! APK Signature Scheme v2 with ECDSA P-256 / SHA-256, which every Android
//! version the host supports (API 24+) verifies, so no JAR (v1) signature is
//! written. Keys are PKCS#8 DER; certificates are X.509 DER.

use crate::zip;
use ring::rand::SystemRandom;
use ring::signature::{EcdsaKeyPair, KeyPair, ECDSA_P256_SHA256_ASN1_SIGNING};
use sha2::{Digest, Sha256};
use std::fmt;
use std::path::Path;

const CHUNK: usize = 1024 * 1024;
const APK_SIGNATURE_SCHEME_V2_ID: u32 = 0x7109_871a;
const SIGNATURE_ECDSA_WITH_SHA256: u32 = 0x0201;
const BLOCK_MAGIC: &[u8; 16] = b"APK Sig Block 42";

#[derive(Debug)]
pub struct SignError(pub String);

impl fmt::Display for SignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "signing: {}", self.0)
    }
}

impl std::error::Error for SignError {}

impl From<zip::ZipError> for SignError {
    fn from(error: zip::ZipError) -> Self {
        Self(error.to_string())
    }
}

/// A signing identity: an ECDSA P-256 key and its certificate.
pub struct Signer {
    key: EcdsaKeyPair,
    certificate: Vec<u8>,
}

impl Signer {
    pub fn new(pkcs8: &[u8], certificate: Vec<u8>) -> Result<Self, SignError> {
        let key =
            EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, pkcs8, &SystemRandom::new())
                .map_err(|e| SignError(format!("the key is not a PKCS#8 ECDSA P-256 key: {e}")))?;
        Ok(Self { key, certificate })
    }

    /// Loads `<dir>/debug.pk8` and `<dir>/debug.der`, creating a new
    /// self-signed debug identity there the first time (like Android
    /// tooling's debug keystore). Android only updates an installed app
    /// signed with the same key, so the identity is kept between builds.
    pub fn debug(dir: &Path) -> Result<Self, SignError> {
        let key_path = dir.join("debug.pk8");
        let cert_path = dir.join("debug.der");
        if !key_path.exists() || !cert_path.exists() {
            let rng = SystemRandom::new();
            let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng)
                .map_err(|e| SignError(format!("key generation failed: {e}")))?;
            let key =
                EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, pkcs8.as_ref(), &rng)
                    .map_err(|e| SignError(e.to_string()))?;
            let certificate = self_signed_certificate(&key, "Rustolonia Android Debug")?;
            std::fs::create_dir_all(dir)
                .and_then(|_| std::fs::write(&key_path, pkcs8.as_ref()))
                .and_then(|_| std::fs::write(&cert_path, &certificate))
                .map_err(|e| {
                    SignError(format!(
                        "cannot store the debug key in {}: {e}",
                        dir.display()
                    ))
                })?;
        }
        let read = |path: &Path| {
            std::fs::read(path).map_err(|e| SignError(format!("{}: {e}", path.display())))
        };
        Self::new(&read(&key_path)?, read(&cert_path)?)
    }

    fn public_key_info(&self) -> Vec<u8> {
        subject_public_key_info(self.key.public_key().as_ref())
    }

    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, SignError> {
        self.key
            .sign(&SystemRandom::new(), message)
            .map(|signature| signature.as_ref().to_vec())
            .map_err(|e| SignError(e.to_string()))
    }
}

/// Inserts an APK Signing Block with a v2 signature into `apk`, a ZIP
/// without one, and returns the signed archive.
pub fn sign_v2(apk: &[u8], signer: &Signer) -> Result<Vec<u8>, SignError> {
    let layout = zip::layout(apk)?;
    let entries = &apk[..layout.central_directory];
    let central = &apk[layout.central_directory..layout.eocd];
    let eocd = &apk[layout.eocd..];

    let digest = content_digest(&[entries, central, eocd]);

    let mut digests = Vec::new();
    push_prefixed(&mut digests, &{
        let mut item = SIGNATURE_ECDSA_WITH_SHA256.to_le_bytes().to_vec();
        push_prefixed(&mut item, &digest);
        item
    });
    let mut certificates = Vec::new();
    push_prefixed(&mut certificates, &signer.certificate);

    let mut signed_data = Vec::new();
    push_prefixed(&mut signed_data, &digests);
    push_prefixed(&mut signed_data, &certificates);
    push_prefixed(&mut signed_data, &[]); // additional attributes

    let mut signatures = Vec::new();
    push_prefixed(&mut signatures, &{
        let mut item = SIGNATURE_ECDSA_WITH_SHA256.to_le_bytes().to_vec();
        push_prefixed(&mut item, &signer.sign(&signed_data)?);
        item
    });

    let mut signer_block = Vec::new();
    push_prefixed(&mut signer_block, &signed_data);
    push_prefixed(&mut signer_block, &signatures);
    push_prefixed(&mut signer_block, &signer.public_key_info());

    let mut signers = Vec::new();
    push_prefixed(&mut signers, &signer_block);
    let mut v2_value = Vec::new();
    push_prefixed(&mut v2_value, &signers);

    // ID-value pair, then the block's size (excluding its own first field)
    // before and after the pairs, then the magic.
    let mut pair = Vec::new();
    pair.extend_from_slice(&((4 + v2_value.len()) as u64).to_le_bytes());
    pair.extend_from_slice(&APK_SIGNATURE_SCHEME_V2_ID.to_le_bytes());
    pair.extend_from_slice(&v2_value);
    let block_size = (pair.len() + 8 + 16) as u64;
    let mut block = Vec::with_capacity(block_size as usize + 8);
    block.extend_from_slice(&block_size.to_le_bytes());
    block.extend_from_slice(&pair);
    block.extend_from_slice(&block_size.to_le_bytes());
    block.extend_from_slice(BLOCK_MAGIC);

    let mut out = Vec::with_capacity(apk.len() + block.len());
    out.extend_from_slice(entries);
    out.extend_from_slice(&block);
    out.extend_from_slice(central);
    let mut eocd = eocd.to_vec();
    let central_offset = (layout.central_directory + block.len()) as u32;
    eocd[16..20].copy_from_slice(&central_offset.to_le_bytes());
    out.extend_from_slice(&eocd);
    Ok(out)
}

/// The v2 content digest: every section is split into 1 MiB chunks, each
/// hashed as `0xa5 || len || chunk`; the result hashes `0x5a || count ||
/// chunk digests`. The EOCD is hashed as it was before the signing block was
/// inserted, which is how `apk` is given here.
fn content_digest(sections: &[&[u8]]) -> Vec<u8> {
    let mut chunk_digests = Vec::new();
    let mut count = 0u32;
    for section in sections {
        for chunk in section.chunks(CHUNK) {
            let mut hasher = Sha256::new();
            hasher.update([0xa5]);
            hasher.update((chunk.len() as u32).to_le_bytes());
            hasher.update(chunk);
            chunk_digests.extend_from_slice(&hasher.finalize());
            count += 1;
        }
    }
    let mut hasher = Sha256::new();
    hasher.update([0x5a]);
    hasher.update(count.to_le_bytes());
    hasher.update(&chunk_digests);
    hasher.finalize().to_vec()
}

fn push_prefixed(out: &mut Vec<u8>, value: &[u8]) {
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value);
}

// --- Minimal DER for a self-signed ECDSA P-256 certificate ------------------

const OID_EC_PUBLIC_KEY: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01];
const OID_PRIME256V1: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07];
const OID_ECDSA_WITH_SHA256: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02];
const OID_COMMON_NAME: &[u8] = &[0x55, 0x04, 0x03];

fn der(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    let len = content.len();
    if len < 0x80 {
        out.push(len as u8);
    } else {
        let bytes: Vec<u8> = len
            .to_be_bytes()
            .into_iter()
            .skip_while(|&b| b == 0)
            .collect();
        out.push(0x80 | bytes.len() as u8);
        out.extend_from_slice(&bytes);
    }
    out.extend_from_slice(content);
    out
}

fn sequence(parts: &[Vec<u8>]) -> Vec<u8> {
    der(0x30, &parts.concat())
}

fn subject_public_key_info(point: &[u8]) -> Vec<u8> {
    let mut bits = vec![0];
    bits.extend_from_slice(point);
    sequence(&[
        sequence(&[der(0x06, OID_EC_PUBLIC_KEY), der(0x06, OID_PRIME256V1)]),
        der(0x03, &bits),
    ])
}

fn self_signed_certificate(key: &EcdsaKeyPair, common_name: &str) -> Result<Vec<u8>, SignError> {
    let name = sequence(&[der(
        0x31,
        &sequence(&[
            der(0x06, OID_COMMON_NAME),
            der(0x0c, common_name.as_bytes()),
        ]),
    )]);
    let algorithm = sequence(&[der(0x06, OID_ECDSA_WITH_SHA256)]);
    let mut serial = [0u8; 9];
    ring::rand::SecureRandom::fill(&SystemRandom::new(), &mut serial[1..])
        .map_err(|e| SignError(e.to_string()))?;
    let tbs = sequence(&[
        der(0xa0, &der(0x02, &[2])), // v3
        der(0x02, &serial),          // positive: leading zero byte
        algorithm.clone(),
        name.clone(),
        sequence(&[der(0x18, b"20000101000000Z"), der(0x18, b"20991231235959Z")]),
        name,
        subject_public_key_info(key.public_key().as_ref()),
    ]);
    let signature = key
        .sign(&SystemRandom::new(), &tbs)
        .map_err(|e| SignError(e.to_string()))?;
    let mut bits = vec![0];
    bits.extend_from_slice(signature.as_ref());
    Ok(sequence(&[tbs, algorithm, der(0x03, &bits)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_prefixed(data: &[u8], at: &mut usize) -> Vec<u8> {
        let len = u32::from_le_bytes(data[*at..*at + 4].try_into().unwrap()) as usize;
        *at += 4;
        let value = data[*at..*at + len].to_vec();
        *at += len;
        value
    }

    #[test]
    fn signing_block_sits_before_the_central_directory_and_verifies() {
        let dir = std::env::temp_dir().join(format!("rustolonia-apk-sign-{}", std::process::id()));
        let signer = Signer::debug(&dir).unwrap();
        let unsigned = zip::write(&[zip::Entry::new("a.txt", b"hello", false)]);
        let signed = sign_v2(&unsigned, &signer).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        // Entries survive and the archive still parses through the new offset.
        assert_eq!(zip::read(&signed).unwrap()[0].contents().unwrap(), b"hello");
        let layout = zip::layout(&signed).unwrap();
        assert_eq!(
            &signed[layout.central_directory - 16..layout.central_directory],
            BLOCK_MAGIC
        );

        // Recompute the digest the way a verifier does and check the signature.
        let block_size = u64::from_le_bytes(
            signed[layout.central_directory - 24..layout.central_directory - 16]
                .try_into()
                .unwrap(),
        ) as usize;
        let block_start = layout.central_directory - block_size - 8;
        let mut eocd = signed[layout.eocd..].to_vec();
        eocd[16..20].copy_from_slice(&(block_start as u32).to_le_bytes());
        let digest = content_digest(&[
            &signed[..block_start],
            &signed[layout.central_directory..layout.eocd],
            &eocd,
        ]);

        let mut at = block_start + 8 + 8 + 4; // size, pair length, pair id
        let signers = read_prefixed(&signed, &mut at);
        let mut at = 0;
        let signer_block = read_prefixed(&signers, &mut at);
        let mut at = 0;
        let signed_data = read_prefixed(&signer_block, &mut at);
        let signatures = read_prefixed(&signer_block, &mut at);
        let public_key = read_prefixed(&signer_block, &mut at);
        assert!(signed_data
            .windows(digest.len())
            .any(|window| window == digest.as_slice()));

        let mut at = 0;
        let signature = read_prefixed(&signatures, &mut at);
        let raw_signature = &signature[8..];
        // The SubjectPublicKeyInfo ends with the uncompressed point.
        let point = &public_key[public_key.len() - 65..];
        ring::signature::UnparsedPublicKey::new(&ring::signature::ECDSA_P256_SHA256_ASN1, point)
            .verify(&signed_data, raw_signature)
            .expect("signature verifies");
    }
}
