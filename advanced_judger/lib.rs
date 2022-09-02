#![cfg_attr(not(feature = "std"), no_std)]

use pink_extension as pink;

mod submittable {
    use pink_utils::attestation::{Attestation, Verifier};
    use ink_env::AccountId;
    use ink_lang as ink;
    use ink_prelude::string::String;
    use ink_prelude::vec::Vec;

    #[openbrush::trait_definition(mock = mock_oracle::MockOracle)]
    pub trait SubmittableOracle {
        #[ink(message)]
        fn admin(&self) -> AccountId;

        #[ink(message)]
        fn verifier(&self) -> Verifier;

        #[ink(message)]
        fn attest(&self, arg: String) -> Result<Attestation, Vec<u8>>;
    }

    #[openbrush::wrapper]
    pub type SubmittableOracleRef = dyn SubmittableOracle;

    // Only used for test, but we have to define it outside `mod tests`
    pub mod mock_oracle {
        use super::*;
        use pink_utils::attestation::{self, Generator};

        pub struct MockOracle {
            admin: AccountId,
            generator: Generator,
            verifier: Verifier,
            should_return_err: bool,
        }

        impl MockOracle {
            pub fn new(admin: AccountId, err: bool) -> Self {
                let (generator, verifier) = attestation::create(b"test");
                MockOracle {
                    admin,
                    generator,
                    verifier,
                    should_return_err: err,
                }
            }

            pub fn admin(&self) -> AccountId {
                self.admin.clone()
            }

            pub fn verifier(&self) -> Verifier {
                self.verifier.clone()
            }

            pub fn attest(&self, _arg: String) -> Result<Attestation, Vec<u8>> {
                if self.should_return_err {
                    Err(Default::default())
                } else {
                    Ok(self.generator.sign(()))
                }
            }
        }
    }
}

#[pink::contract(env=PinkEnvironment)]
mod advanced_judger {
    use super::pink::PinkEnvironment;
    use pink_utils::attestation;
    use ink_lang as ink;
    use ink_prelude::string::String;
    use ink_storage::traits::SpreadAllocate;
    use ink_storage::Mapping;
    use scale::{Decode, Encode};

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct AdvancedJudger {
        admin: AccountId,
        badge_contract_options: Option<(AccountId, u32)>,
        attestation_verifier: attestation::Verifier,
        attestation_generator: attestation::Generator,
        passed_contracts: Mapping<AccountId, ()>,
    }

    /// Errors that can occur upon calling this contract.
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        BadOrigin,
        BadgeContractNotSetUp,
        FailedToIssueBadge,
        FailedToVerify,
        InvalidParameter,
        AlreadySubmitted,
    }

    /// Type alias for the contract's result type.
    pub type Result<T> = core::result::Result<T, Error>;

    impl AdvancedJudger {
        #[ink(constructor)]
        pub fn new() -> Self {
            // Create the attestation helpers
            let (generator, verifier) = attestation::create(b"adv-challenge-attestation-key");
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
        /// The attestation must be created by [attest_gist] function. After the verification of
        /// the attestation, the the sender account will the linked to a Github username. Then a
        /// POAP redemption code will be allocated to the sender.
        ///
        /// Each blockchain account and github account can only be linked once.
        #[ink(message)]
        pub fn redeem(&mut self, attestation: attestation::Attestation) -> Result<()> {
            // Verify the attestation
            let data: GoodSubmission = self
                .attestation_verifier
                .verify_as(&attestation)
                .ok_or(Error::FailedToVerify)?;
            // The caller must be the attested account
            if data.admin != self.env().caller() {
                return Err(Error::BadOrigin);
            }
            // The contract is not submitted twice
            if self.passed_contracts.contains(data.contract) {
                return Err(Error::AlreadySubmitted);
            }
            self.passed_contracts.insert(data.contract, &());

            // Issue the badge
            let (contract, id) = self
                .badge_contract_options
                .ok_or(Error::BadgeContractNotSetUp)?;

            use fat_badges::issuable::IssuableRef;
            let badges: &IssuableRef = &contract;
            let r = badges.issue(id, data.admin);
            r.or(Err(Error::FailedToIssueBadge))
        }

        // Queries

        /// Attests a contract submission has passed the check (Query only)
        ///
        /// Call the submitted contract with an URL, and check that it can produce a valid offchain
        /// attestation. Once the check is passed, it returns an attestation that can be used
        /// to redeem a badge by `Self::redeem` by the admin of the submitted contract.
        #[ink(message)]
        pub fn check_contract(
            &self,
            contract: AccountId,
            url: String,
        ) -> Result<attestation::Attestation> {
            use crate::submittable::SubmittableOracleRef;
            let oracle: &SubmittableOracleRef = &contract;

            // The attestation should be at least `Ok(attestation)`
            let attestation = oracle.attest(url).or(Err(Error::FailedToVerify))?;

            // The attestation can be verified successfully
            let verifier = oracle.verifier();
            if !verifier.verify(&attestation) {
                return Err(Error::FailedToVerify);
            }

            // Ok. Now we can produce the attestation to redeem
            let admin = oracle.admin();
            let quote = GoodSubmission { admin, contract };
            let result = self.attestation_generator.sign(quote);
            Ok(result)
        }
    }

    #[derive(Clone, Encode, Decode, Debug)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    struct GoodSubmission {
        admin: AccountId,
        contract: AccountId,
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::submittable::{mock_oracle::MockOracle, mock_submittableoracle};

        use ink_lang as ink;

        fn default_accounts() -> ink_env::test::DefaultAccounts<PinkEnvironment> {
            ink_env::test::default_accounts::<Environment>()
        }

        #[ink::test]
        fn end_to_end() {
            pink_extension_runtime::mock_ext::mock_all_ext();

            // Test accounts
            let accounts = default_accounts();

            use fat_badges::issuable::mock_issuable;
            use openbrush::traits::mock::{Addressable, SharedCallStack};

            let stack = SharedCallStack::new(accounts.alice);
            mock_issuable::using(stack.clone(), || {
                mock_submittableoracle::using(stack.clone(), || {
                    // let badges = Addressable::create_native(1, fat_badges::FatBadges::new(), stack.clone());

                    let badges = mock_issuable::deploy(fat_badges::FatBadges::new());
                    // Deploy the mock oracle on behalf of Bob
                    let good_oracle =
                        mock_submittableoracle::deploy(MockOracle::new(accounts.bob, false));
                    let bad_oracle =
                        mock_submittableoracle::deploy(MockOracle::new(accounts.bob, true));
                    let contract =
                        Addressable::create_native(1, AdvancedJudger::new(), stack.clone());

                    // Create a badge and set the oracle as its issuer
                    let id = badges
                        .call_mut()
                        .new_badge("test-badge".to_string())
                        .unwrap();
                    badges
                        .call_mut()
                        .add_code(id, vec!["code1".to_string(), "code2".to_string()])
                        .unwrap();
                    badges.call_mut().add_issuer(id, contract.id()).unwrap();
                    contract.call_mut().config_issuer(badges.id(), id).unwrap();

                    // Test the happy path
                    stack.switch_account(accounts.bob).unwrap();
                    let att = contract
                        .call()
                        .check_contract(good_oracle.id(), "some-url".to_string())
                        .expect("good contract must pass the check");

                    let data: GoodSubmission = contract
                        .call()
                        .attestation_verifier
                        .verify_as(&att)
                        .expect("should pass verification");
                    assert_eq!(data.admin, accounts.bob);
                    // Bob can redeem the code
                    contract.call_mut().redeem(att).unwrap();
                    // Bob has received the POAP
                    assert_eq!(badges.call().get(id), Ok("code1".to_string()));

                    // Test the bad path
                    assert_eq!(
                        contract
                            .call()
                            .check_contract(bad_oracle.id(), "some-url".to_string()),
                        Err(Error::FailedToVerify)
                    );
                });
            });
        }
    }
}
