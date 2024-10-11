use ed25519_dalek::{Keypair, PublicKey as EdPublicKey, SecretKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;
use serde::de::{Error as DeError, Visitor};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PublicKey(EdPublicKey);

impl PublicKey {
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
    pub fn genesis() -> Self {
        // Create a deterministic public key for the genesis block
        let bytes = [0u8; 32]; // All zeros for simplicity
        PublicKey(EdPublicKey::from_bytes(&bytes).expect("Invalid genesis public key"))
    }
}

impl Hash for PublicKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_bytes().hash(state);
    }
}

impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(self.0.as_bytes())
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PublicKeyVisitor;

        impl<'de> Visitor<'de> for PublicKeyVisitor {
            type Value = PublicKey;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a 32-byte public key")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: DeError,
            {
                if v.len() != 32 {
                    return Err(E::custom("public key must be 32 bytes"));
                }
                EdPublicKey::from_bytes(v)
                    .map(PublicKey)
                    .map_err(|e| E::custom(format!("invalid public key: {}", e)))
            }
        }

        deserializer.deserialize_bytes(PublicKeyVisitor)
    }
}

pub struct KeyPair(Keypair);

impl KeyPair {
    pub fn generate() -> Self {
        let mut csprng = OsRng {};
        KeyPair(Keypair::generate(&mut csprng))
    }

    pub fn from_secret_key(secret: &[u8]) -> Result<Self, ed25519_dalek::SignatureError> {
        let secret_key = SecretKey::from_bytes(secret)?;
        let public_key: EdPublicKey = (&secret_key).into();
        Ok(KeyPair(Keypair {
            secret: secret_key,
            public: public_key,
        }))
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.0.public)
    }

    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.0.sign(message).to_bytes().to_vec()
    }
}

pub fn verify_signature(public_key: &PublicKey, message: &[u8], signature: &[u8]) -> bool {
    if let Ok(sig) = Signature::from_bytes(signature) {
        public_key.0.verify(message, &sig).is_ok()
    } else {
        false
    }
}
