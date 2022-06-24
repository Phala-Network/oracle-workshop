#![cfg_attr(not(feature = "std"), no_std)]
#![feature(trace_macros)]

use pink_extension as pink;

#[pink::contract(env=PinkEnvironment)]
mod easy_oracle {
    use super::pink;
    use pink::logger::{Level, Logger};
    use pink::{http_get, PinkEnvironment};

    use fat_utils::attestation;
    use ink_prelude::{
        string::{String, ToString},
        vec::Vec,
    };
    use ink_storage::traits::SpreadAllocate;
    use ink_storage::Mapping;
    use scale::{Decode, Encode};

    use fat_badges::issuable::IssuableRef;

    static LOGGER: Logger = Logger::with_max_level(Level::Info);
    pink::register_logger!(&LOGGER);

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct EasyOracle {
        admin: AccountId,
        badge_contract_options: Option<(AccountId, u32)>,
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
        #[ink(message)]
        pub fn config_issuer(&mut self, contract: AccountId, badge_id: u32) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::BadOrigin);
            }
            // Create a reference to the already deployed FatBadges contract
            self.badge_contract_options = Some((contract, badge_id));
            Ok(())
        }

        /// Redeems a POAP with a signed `attestation`. (callable)
        ///
        /// The attestation must be created by [`attest_gist`] function. After the verification of
        /// the attestation, the the sender account will the linked to a Github username. Then a
        /// POAP redemption code will be allocated to the sender.
        ///
        /// Each blockchain account and github account can only be linked once.
        #[ink(message)]
        pub fn redeem(&mut self, attestation: attestation::Attestation) -> Result<()> {
            // Verify the attestation
            let data: GistQuote = self
                .attestation_verifier
                .verify_as(&attestation)
                .ok_or(Error::InvalidSignature)?;
            // The caller must be the attested account
            if data.account_id != self.env().caller() {
                pink::warn!("No permission.");
                return Err(Error::NoPermission);
            }

            if self.linked_users.contains(data.username) {
                pink::warn!("Username alreay in use.");
                return Err(Error::UsernameAlreadyInUse);
            }

            let (contract, id) = self
                .badge_contract_options
                .as_mut()
                .ok_or(Error::BadgeContractNotSetUp)?;
            pink::warn!("Got badge contract. Calling...");

            let badges: &IssuableRef = contract;
            let result = badges.issue(*id, data.account_id);
            pink::warn!("Badges.issue() result = {:?}", result);
            result.or(Err(Error::FailedToIssueBadge))
        }

        // Queries

        /// Attests a Github Gist by the raw file url. (Query only)
        ///
        /// It sends a HTTPS request to the url and extract an address from the claim ("This gist
        /// is owned by address: 0x..."). Once the claim is verified, it returns a signed
        /// attestation with the data `(username, account_id)`.
        ///
        /// The `Err` variant of the result is an encoded `Error` to simplify cross-contract calls.
        /// Particularly, when another contract wants to call us, they may not want to depend on
        /// any special type defined by us (`Error` in this case). So we only return generic types.
        #[ink(message)]
        pub fn attest(
            &self,
            url: String,
        ) -> core::result::Result<attestation::Attestation, Vec<u8>> {
            // Verify the URL
            let gist_url = parse_gist_url(&url).map_err(|e| e.encode())?;
            // Fetch the gist content
            let resposne = http_get!(url);
            if resposne.status_code != 200 {
                return Err(Error::RequestFailed.encode());
            }
            let body = resposne.body;
            // Verify the claim and extract the account id
            let account_id = extract_claim(&body).map_err(|e| e.encode())?;
            let quote = GistQuote {
                username: gist_url.username,
                account_id,
            };
            let result = self.attestation_generator.sign(quote);
            Ok(result)
        }

        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin.clone()
        }

        /// The attestation verifier
        #[ink(message)]
        pub fn verifier(&self) -> attestation::Verifier {
            self.attestation_verifier.clone()
        }

        /// Helper query to return the account id of the current contract instance
        #[ink(message)]
        pub fn get_id(&self) -> AccountId {
            self.env().account_id()
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct GistUrl {
        username: String,
        gist_id: String,
        filename: String,
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

            use fat_badges::issuable::mock_issuable;
            use openbrush::traits::mock::{Addressable, SharedCallStack};

            let stack = SharedCallStack::new(accounts.alice);
            mock_issuable::using(stack.clone(), || {
                // Deploy a FatBadges contract
                let badges = mock_issuable::deploy(fat_badges::FatBadges::new());

                // Construct our contract (deployed by `accounts.alice` by default)
                let contract = Addressable::create_native(1, EasyOracle::new(), stack);

                // Create a badge and add the oracle contract as its issuer
                let id = badges
                    .call_mut()
                    .new_badge("test-badge".to_string())
                    .unwrap();
                assert!(badges
                    .call_mut()
                    .add_code(id, vec!["code1".to_string(), "code2".to_string()])
                    .is_ok());
                assert!(badges.call_mut().add_issuer(id, contract.id()).is_ok());
                // Tell the oracle the badges are ready to issue
                assert!(contract.call_mut().config_issuer(badges.id(), id).is_ok());

                // Generate an attestation
                //
                // Mock a http request first (the 256 bits account id is the pubkey of Alice)
                mock::mock_http_request(|_| {
                    HttpResponse::ok(b"This gist is owned by address: 0x0101010101010101010101010101010101010101010101010101010101010101".to_vec())
                });
                let result = contract.call().attest("https://gist.githubusercontent.com/h4x3rotab/0cabeb528bdaf30e4cf741e26b714e04/raw/620f958fb92baba585a77c1854d68dc986803b4e/test%2520gist".to_string());
                assert!(result.is_ok());

                let attestation = result.unwrap();
                let data: GistQuote = Decode::decode(&mut &attestation.data[..]).unwrap();
                assert_eq!(data.username, "h4x3rotab");
                assert_eq!(data.account_id, accounts.alice);

                // Before redeem
                assert!(badges.call().get(id).is_err());

                // Redeem and check if the contract as the code distributed
                contract
                    .call_mut()
                    .redeem(attestation)
                    .expect("Should be able to issue badge");
                assert_eq!(badges.call().get(id), Ok("code1".to_string()));
            });
        }
    }
}
