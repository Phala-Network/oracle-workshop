#![cfg_attr(not(feature = "std"), no_std)]

use pink_extension as pink;

#[pink::contract(env=PinkEnvironment)]
mod easy_oracle {
    use super::pink::{http_get, PinkEnvironment};
    use crate::utils::attestation;
    use ink_prelude::{string::String, vec::Vec};
    use ink_storage::traits::SpreadAllocate;
    use ink_storage::Mapping;
    use scale::{Decode, Encode};

    use fat_badges::FatBadgesRef;

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct EasyOracle {
        admin: AccountId,
        badge_contract_options: Option<(FatBadgesRef, u32)>,
        attestation_verifier: attestation::Verifier,
        attestation_generator: attestation::Generator,
        linked_users: Mapping<String, ()>,
    }

    /// Errors that can occur upon calling this contract.
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        BadOrigin,
        BadgeContractNotSetUp,
        InvalidUrl,
        RequestFailed,
        NoClaimFound,
        InvalidAddressLength,
        InvalidAddress,
        NoPermission,
        InvalidSignature,
        UsernameAlreadyInUse,
        AccountAlreadyInUse,
        FailedToIssueBadge,
    }

    /// Type alias for the contract's result type.
    pub type Result<T> = core::result::Result<T, Error>;

    #[derive(PartialEq, Eq, Debug)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    struct GistUrl {
        username: String,
        gist_id: String,
        filename: String,
    }

    impl EasyOracle {
        #[ink(constructor)]
        pub fn new() -> Self {
            // Create the attestation helpers
            let (generator, verifier) = attestation::create(b"gist-attestation-key");
            // Save sender as the contract admin
            let admin = Self::env().caller();

            ink_lang::utils::initialize_contract(|this: &mut Self| {
                this.admin = admin;
                this.badge_contract_options = None;
                this.attestation_generator = generator;
                this.attestation_verifier = verifier;
            })
        }

        // Commands

        /// Sets the downstream badge contract
        ///
        /// Only the admin can call it.
        pub fn config_issuer(&mut self, contract: AccountId, badge_id: u32) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::BadOrigin);
            }
            // Create a reference to the already deployed FatBadges contract
            use ink_env::call::FromAccountId;
            let contract_ref = FatBadgesRef::from_account_id(contract);
            self.badge_contract_options = Some((contract_ref, badge_id));
            Ok(())
        }

        /// Redeems a POAP with a signed `attestation`. (callable)
        ///
        /// The attestation must be created by [attest_gist] function. After the verification of
        /// the attestation, the the sender account will the linked to a Github username. Then a
        /// POAP redemption code will be allocated to the sender.
        ///
        /// Each blockchain account and github account can only be linked once.
        #[ink(message)]
        pub fn redeem(&mut self, attestation: attestation::Attestation<GistQuote>) -> Result<()> {
            // Verify the attestation
            if !self.attestation_verifier.verify(&attestation) {
                return Err(Error::InvalidSignature);
            }
            let data = attestation.data;
            // The caller must be the attested account
            if data.account_id != self.env().caller() {
                return Err(Error::NoPermission);
            }

            let username = data.username;
            let account = data.account_id;

            if self.linked_users.contains(username) {
                return Err(Error::UsernameAlreadyInUse);
            }

            let (contract, id) = self
                .badge_contract_options
                .as_mut()
                .ok_or(Error::BadgeContractNotSetUp)?;

            #[cfg(not(test))]
            contract
                .issue(*id, account)
                .or(Err(Error::FailedToIssueBadge))?;
            #[cfg(test)]
            {
                tests::with_badges_contract(|fat_badges| fat_badges.issue(*id, account))
                    .or(Err(Error::FailedToIssueBadge))?;
            }

            Ok(())
        }

        // Queries

        /// Attests a Github Gist by the raw file url. (Query only)
        ///
        /// It sends a HTTPS request to the url and extract an address from the claim ("This gist
        /// is owned by address: 0x..."). Once the claim is verified, it returns a signed
        /// attestation with the pair `(github_username, account_id)`.
        #[ink(message)]
        pub fn attest_gist(&self, url: String) -> Result<attestation::Attestation<GistQuote>> {
            // Verify the URL
            let gist_url = parse_gist_url(&url)?;
            // Fetch the gist content
            let resposne = http_get!(url);
            if resposne.status_code != 200 {
                return Err(Error::RequestFailed);
            }
            let body = resposne.body;
            // Verify the claim and extract the account id
            let account_id = extract_claim(&body)?;
            let quote = GistQuote {
                username: gist_url.username,
                account_id,
            };
            let result = self.attestation_generator.sign(quote);
            Ok(result)
        }

        /// Helper query to return the account id of the current contract instance
        #[ink(message)]
        pub fn get_id(&self) -> AccountId {
            self.env().account_id()
        }
    }

    #[derive(Clone, Encode, Decode, Debug)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct GistQuote {
        username: String,
        account_id: AccountId,
    }

    /// Parses a Github Gist url.
    ///
    /// - Returns a parsed [GistUrl] struct if the input is a valid url;
    /// - Otherwise returns an [Error].
    fn parse_gist_url(url: &str) -> Result<GistUrl> {
        let path = url
            .strip_prefix("https://gist.githubusercontent.com/")
            .ok_or(Error::InvalidUrl)?;
        let components: Vec<_> = path.split('/').collect();
        if components.len() < 5 {
            return Err(Error::InvalidUrl);
        }
        Ok(GistUrl {
            username: components[0].to_string(),
            gist_id: components[1].to_string(),
            filename: components[4].to_string(),
        })
    }

    const CLAIM_PREFIX: &str = "This gist is owned by address: 0x";
    const ADDRESS_LEN: usize = 64;

    /// Extracts the ownerhip of the gist from a claim in the gist body.
    ///
    /// A valid claim must have the statement "This gist is owned by address: 0x..." in `body`. The
    /// address must be the 256 bits public key of the Substrate account in hex.
    ///
    /// - Returns a 256-bit `AccountId` representing the owner account if the claim is valid;
    /// - otherwise returns an [Error].
    fn extract_claim(body: &[u8]) -> Result<AccountId> {
        let body = String::from_utf8_lossy(body);
        let pos = body.find(CLAIM_PREFIX).ok_or(Error::NoClaimFound)?;
        let addr: String = body
            .chars()
            .skip(pos)
            .skip(CLAIM_PREFIX.len())
            .take(ADDRESS_LEN)
            .collect();
        let addr = addr.as_bytes();
        let account_id = decode_accountid_256(addr)?;
        Ok(account_id)
    }

    /// Decodes a hex string as an 256-bit AccountId32
    fn decode_accountid_256(addr: &[u8]) -> Result<AccountId> {
        use hex::FromHex;
        if addr.len() != ADDRESS_LEN {
            return Err(Error::InvalidAddressLength);
        }
        let bytes = <[u8; 32]>::from_hex(addr).or(Err(Error::InvalidAddress))?;
        Ok(AccountId::from(bytes))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink_lang as ink;

        fn default_accounts() -> ink_env::test::DefaultAccounts<PinkEnvironment> {
            ink_env::test::default_accounts::<Environment>()
        }

        fn set_next_caller(caller: AccountId) {
            ink_env::test::set_caller::<Environment>(caller);
        }

        thread_local!(pub static TEST_HARNESS: std::cell::RefCell<Option<fat_badges::FatBadges>> = Default::default());

        /// Initializes a FatBadges contract and return its address
        fn init_mock_badge_contract() -> ink_env::AccountId {
            TEST_HARNESS.with(|fat_badges| {
                let badges_contract = fat_badges::FatBadges::new();
                let contract_id = badges_contract.get_id();
                *fat_badges.borrow_mut() = Some(badges_contract);
                contract_id
            })
        }

        /// Test harness to get contracts
        pub fn with_badges_contract<F, R>(f: F) -> R
        where
            F: FnOnce(&mut fat_badges::FatBadges) -> R,
        {
            TEST_HARNESS.with(|fat_badges| {
                let mut badges_contract_ref = fat_badges.borrow_mut();
                let badges_contract = badges_contract_ref.as_mut().unwrap();
                f(badges_contract)
            })
        }

        #[ink::test]
        fn can_parse_gist_url() {
            let result = parse_gist_url("https://gist.githubusercontent.com/h4x3rotab/0cabeb528bdaf30e4cf741e26b714e04/raw/620f958fb92baba585a77c1854d68dc986803b4e/test%2520gist");
            assert_eq!(
                result,
                Ok(GistUrl {
                    username: "h4x3rotab".to_string(),
                    gist_id: "0cabeb528bdaf30e4cf741e26b714e04".to_string(),
                    filename: "test%2520gist".to_string(),
                })
            );
            let err = parse_gist_url("http://example.com");
            assert_eq!(err, Err(Error::InvalidUrl));
        }

        #[ink::test]
        fn can_decode_claim() {
            let ok = extract_claim(b"...This gist is owned by address: 0x0123456789012345678901234567890123456789012345678901234567890123...");
            assert_eq!(
                ok,
                decode_accountid_256(
                    b"0123456789012345678901234567890123456789012345678901234567890123"
                )
            );
            // Bad cases
            assert_eq!(
                extract_claim(b"This gist is owned by"),
                Err(Error::NoClaimFound),
            );
            assert_eq!(
                extract_claim(b"This gist is owned by address: 0xAB"),
                Err(Error::InvalidAddressLength),
            );
            assert_eq!(
                extract_claim(b"This gist is owned by address: 0xXX23456789012345678901234567890123456789012345678901234567890123"),
                Err(Error::InvalidAddress),
            );
        }

        #[ink::test]
        fn end_to_end() {
            use pink_extension::chain_extension::{mock, HttpResponse};

            // Mock derive key call (a pregenerated key pair)
            mock::mock_derive_sr25519_key(|_| {
                hex::decode("78003ee90ff2544789399de83c60fa50b3b24ca86c7512d0680f64119207c80ab240b41344968b3e3a71a02c0e8b454658e00e9310f443935ecadbdd1674c683").unwrap()
            });
            mock::mock_get_public_key(|_| {
                hex::decode("ce786c340288b79a951c68f87da821d6c69abd1899dff695bda95e03f9c0b012")
                    .unwrap()
            });
            mock::mock_sign(|_| b"mock-signature".to_vec());
            mock::mock_verify(|_| true);

            // Test accounts
            let accounts = default_accounts();
            let badges_contract_id = init_mock_badge_contract();

            // Construct a contract (deployed by `accounts.alice` by default)
            let mut contract = EasyOracle::new();

            // Create a badge and set the oracle as its issuer
            let id = with_badges_contract(|badges_contract| {
                let id = badges_contract.new_badge("test-badge".to_string()).unwrap();
                assert!(badges_contract
                    .add_code(id, vec!["code1".to_string(), "code2".to_string()])
                    .is_ok());
                assert!(contract.config_issuer(badges_contract_id, id).is_ok());
                id
            });

            // Generate an attestation
            //
            // Mock a http request first (the 256 bits account id is the pubkey of Alice)
            mock::mock_http_request(|_| {
                HttpResponse::ok(b"This gist is owned by address: 0x0101010101010101010101010101010101010101010101010101010101010101".to_vec())
            });
            let result = contract.attest_gist("https://gist.githubusercontent.com/h4x3rotab/0cabeb528bdaf30e4cf741e26b714e04/raw/620f958fb92baba585a77c1854d68dc986803b4e/test%2520gist".to_string());
            assert!(result.is_ok());

            let attestation = result.unwrap();
            assert_eq!(attestation.data.username, "h4x3rotab");
            assert_eq!(attestation.data.account_id, accounts.alice);

            // Before redeem
            with_badges_contract(|badges_contract| assert!(badges_contract.get(id).is_err()));

            // Redeem
            assert!(contract.redeem(attestation).is_ok());

            with_badges_contract(|badges_contract| {
                assert_eq!(badges_contract.get(id), Ok("code1".to_string()))
            });
        }
    }
}

mod utils;
