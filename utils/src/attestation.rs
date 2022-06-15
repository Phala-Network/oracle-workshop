///! Utilities to create and verify off-chain attestation
use core::fmt;
use ink_prelude::vec::Vec;
use ink_storage::traits::{PackedLayout, SpreadAllocate, SpreadLayout};
use pink::{chain_extension::SigType, derive_sr25519_key, get_public_key, sign, verify};
use pink_extension as pink;
use scale::{Decode, Encode};

/// A signed payload produced by a [`Generator`], and can be validated by [`Verifier`].
#[derive(Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct Attestation {
    pub data: Vec<u8>,
    pub signature: Vec<u8>,
    // TODO: add metadata
}

/// An attestation verifier
#[derive(Debug, Encode, Decode, Clone, SpreadLayout, PackedLayout, SpreadAllocate)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout,)
)]
pub struct Verifier {
    pub pubkey: Vec<u8>,
}

impl Verifier {
    /// Verifies an attestation
    pub fn verify(&self, attestation: &Attestation) -> bool {
        verify!(
            &attestation.data,
            &self.pubkey,
            &attestation.signature,
            SigType::Sr25519
        )
    }

    /// Verifies an attestation and decodes the inner data
    pub fn verify_as<T: Decode>(&self, attestation: &Attestation) -> Option<T> {
        if !self.verify(&attestation) {
            return None;
        }
        Decode::decode(&mut &attestation.data[..]).ok()
    }
}

/// An attestation generator
#[derive(Encode, Decode, Clone, SpreadLayout, PackedLayout, SpreadAllocate)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout,)
)]
pub struct Generator {
    pub privkey: Vec<u8>,
}

impl Generator {
    /// Produces a signed attestation with the given `data`
    pub fn sign<T: Clone + Encode + Decode>(&self, data: T) -> Attestation {
        let encoded = Encode::encode(&data);
        let signature = sign!(&encoded, &self.privkey, SigType::Sr25519);
        Attestation { data: encoded, signature }
    }
}

impl fmt::Debug for Generator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // We don't want to leak the privkey to anyone
        write!(f, "Generator")
    }
}

/// Creates a pair of attestation utility to do off-chain attestation
pub fn create(salt: &[u8]) -> (Generator, Verifier) {
    let privkey = derive_sr25519_key!(salt);
    let pubkey = get_public_key!(&privkey, SigType::Sr25519);
    (Generator { privkey }, Verifier { pubkey })
}

#[cfg(test)]
mod test {
    use super::*;
    use ink_lang as ink;

    #[ink::test]
    fn it_works() {
        use pink_extension::chain_extension::mock;

        // Mock derive key call (a pregenerated key pair)
        mock::mock_derive_sr25519_key(|_| {
            hex::decode("78003ee90ff2544789399de83c60fa50b3b24ca86c7512d0680f64119207c80ab240b41344968b3e3a71a02c0e8b454658e00e9310f443935ecadbdd1674c683").unwrap()
        });
        mock::mock_get_public_key(|_| {
            hex::decode("ce786c340288b79a951c68f87da821d6c69abd1899dff695bda95e03f9c0b012").unwrap()
        });
        mock::mock_sign(|_| b"mock-signature".to_vec());
        mock::mock_verify(|_| true);

        // Generate an attestation and verify it later
        #[derive(Clone, Encode, Decode, scale_info::TypeInfo)]
        struct SomeData {
            x: u32,
        }

        let (generator, verifier) = create(b"salt");
        let attestation = generator.sign(SomeData { x: 123 });
        assert!(verifier.verify(&attestation));
    }
}
