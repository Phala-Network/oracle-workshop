#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

pub use crate::fat_badges::{FatBadges, Result};

// Define a trait for cross-contract call. Necessary to enable it in unit tests.
pub mod issuable {
    use ink_env::AccountId;
    use ink_lang as ink;

    #[openbrush::trait_definition(mock = crate::FatBadges)]
    pub trait Issuable {
        #[ink(message)]
        fn issue(&mut self, id: u32, dest: AccountId) -> crate::Result<()>;
    }

    #[openbrush::wrapper]
    pub type IssuableRef = dyn Issuable;
}

#[openbrush::contract]
mod fat_badges {
    use super::issuable::*;
    use ink_lang::codegen::Env;
    use ink_prelude::{string::String, vec::Vec};
    use ink_storage::traits::{PackedLayout, SpreadAllocate, SpreadLayout};
    use ink_storage::Mapping;
    use scale::{Decode, Encode};

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct FatBadges {
        admin: AccountId,
        total_badges: u32,
        badge_info: Mapping<u32, BadgeInfo>,
        badge_issuers: Mapping<(u32, AccountId), ()>,
        badge_code: Mapping<(u32, u32), String>,
        badge_assignments: Mapping<(u32, AccountId), u32>,
    }

    /// Errors that can occur upon calling this contract.
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        BadOrigin,
        BadgeNotFound,
        NotAnIssuer,
        NotFound,
        RunOutOfCode,
        Duplicated,
    }

    /// Type alias for the contract's result type.
    pub type Result<T> = core::result::Result<T, Error>;

    /// The basic information of a badge
    #[derive(
        Debug, PartialEq, Encode, Decode, Clone, SpreadLayout, PackedLayout, SpreadAllocate,
    )]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout,)
    )]
    pub struct BadgeInfo {
        /// Badge ID
        id: u32,
        /// The admin to manage the badge
        admin: AccountId,
        /// Name of the badge
        name: String,
        /// Total available redeem code
        num_code: u32,
        /// The number of issued badges
        num_issued: u32,
    }

    impl FatBadges {
        #[ink(constructor)]
        pub fn new() -> Self {
            ink_lang::utils::initialize_contract(|this: &mut Self| {
                this.admin = Self::env().caller();
                this.total_badges = 0;
            })
        }

        // Commands

        /// Creates a new badge and become the admin of the badge
        ///
        /// Return the id of the badge.
        #[ink(message)]
        pub fn new_badge(&mut self, name: String) -> Result<u32> {
            let caller = self.env().caller();
            let id = self.total_badges;
            let badge = BadgeInfo {
                id,
                admin: caller,
                name,
                num_code: 0,
                num_issued: 0,
            };
            self.badge_info.insert(id, &badge);
            self.total_badges += 1;
            Ok(id)
        }

        /// Adds a badge issuer
        ///
        /// The caller must be the badge admin.
        #[ink(message)]
        pub fn add_issuer(&mut self, id: u32, issuer: AccountId) -> Result<()> {
            self.ensure_badge_admin(id)?;
            self.badge_issuers.insert((id, issuer), &());
            Ok(())
        }

        /// Removes a badge issuer
        ///
        /// The caller must be the badge admin.
        #[ink(message)]
        pub fn remove_issuer(&mut self, id: u32, issuer: AccountId) -> Result<()> {
            self.ensure_badge_admin(id)?;
            self.badge_issuers.remove((id, issuer));
            Ok(())
        }

        /// Appends a list of redeem code to a badge
        ///
        /// The caller must be the badge admin.
        #[ink(message)]
        pub fn add_code(&mut self, id: u32, code: Vec<String>) -> Result<()> {
            let mut badge = self.ensure_badge_admin(id)?;
            let start = badge.num_code;
            badge.num_code += code.len() as u32;
            for (i, entry) in code.iter().enumerate() {
                let idx = (i as u32) + start;
                self.badge_code.insert((id, idx), entry);
            }
            self.badge_info.insert(id, &badge);
            Ok(())
        }

        // Queries

        /// Returns the number of all the badges
        #[ink(message)]
        pub fn get_total_badges(&self) -> u32 {
            self.total_badges
        }

        /// Returns the detailed information of a badge
        #[ink(message)]
        pub fn get_badge_info(&self, id: u32) -> Result<BadgeInfo> {
            self.badge_info.get(id).ok_or(Error::BadgeNotFound)
        }

        /// Checks if an account is a badge issuer
        #[ink(message)]
        pub fn is_badge_issuer(&self, id: u32, issuer: AccountId) -> bool {
            self.badge_issuers.contains((id, issuer))
        }

        /// Reads the badge code assigned to the caller if exists
        #[ink(message)]
        pub fn get(&self, id: u32) -> Result<String> {
            let caller = self.env().caller();
            let code_idx = self
                .badge_assignments
                .get((id, caller))
                .ok_or(Error::NotFound)?;
            let code = self
                .badge_code
                .get((id, code_idx))
                .expect("Assigned code exists; qed.");
            Ok(code)
        }

        // Helper functions

        /// Returns the badge info if it exists
        fn ensure_badge(&self, id: u32) -> Result<BadgeInfo> {
            self.badge_info.get(id).ok_or(Error::BadgeNotFound)
        }

        /// Returns the badge if the it exists and the caller is the admin
        fn ensure_badge_admin(&self, id: u32) -> Result<BadgeInfo> {
            let caller = self.env().caller();
            let badge = self.badge_info.get(id).ok_or(Error::BadgeNotFound)?;
            if badge.admin != caller {
                return Err(Error::BadOrigin);
            }
            Ok(badge)
        }
    }

    impl Issuable for FatBadges {
        /// Issues a badge to the `dest` account
        ///
        /// The caller must be the badge admin or a badge issuer. Return a `RunOutOfCode` error
        /// when there's no enough redeem code to issue.
        #[ink(message)]
        fn issue(&mut self, id: u32, dest: AccountId) -> Result<()> {
            let caller = self.env().caller();
            let mut badge = self.ensure_badge(id)?;
            // Allow the badge issuers or the badge admin
            if !self.badge_issuers.contains((id, caller)) && caller != badge.admin {
                return Err(Error::NotAnIssuer);
            }
            // Make sure we don't issue more than what we have
            if badge.num_issued >= badge.num_code {
                return Err(Error::RunOutOfCode);
            }
            // No duplication
            if self.badge_assignments.contains((id, dest)) {
                return Err(Error::Duplicated);
            }
            // Update assignment and issued count
            let idx = badge.num_issued;
            self.badge_assignments.insert((id, dest), &idx);
            badge.num_issued += 1;
            self.badge_info.insert(id, &badge);
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink_lang as ink;
        use openbrush::traits::mock::{Addressable, SharedCallStack};

        fn default_accounts() -> ink_env::test::DefaultAccounts<ink_env::DefaultEnvironment> {
            ink_env::test::default_accounts::<Environment>()
        }

        #[ink::test]
        fn issue_badges() {
            let accounts = default_accounts();

            let stack = SharedCallStack::new(accounts.alice);
            let fat_badges = Addressable::create_native(1, FatBadges::new(), stack.clone());
            assert_eq!(fat_badges.call().admin, accounts.alice);

            // Alice can create a badge
            let id = fat_badges
                .call_mut()
                .new_badge("Phala Workshop: Easy".to_string())
                .expect("Should be able to create badges");

            // Can add an issuer
            assert!(fat_badges.call_mut().add_issuer(id, accounts.bob).is_ok());

            // Bob can create another badge
            stack.switch_account(accounts.bob).unwrap();
            let id_adv = fat_badges
                .call_mut()
                .new_badge("Phala Workshop: Advanced".to_string())
                .expect("Should be able to create badges");
            stack.switch_account(accounts.alice).unwrap();
            assert_eq!(
                fat_badges.call_mut().add_issuer(id_adv, accounts.bob),
                Err(Error::BadOrigin),
                "Only the badge owner can add issuers"
            );
            assert_eq!(
                fat_badges.call_mut().add_issuer(999, accounts.bob),
                Err(Error::BadgeNotFound),
                "Non-existing badge"
            );

            // Can remove an issuer
            assert!(fat_badges
                .call_mut()
                .add_issuer(id, accounts.charlie)
                .is_ok());
            assert!(fat_badges
                .call_mut()
                .remove_issuer(id, accounts.charlie)
                .is_ok());
            assert!(!fat_badges.call().is_badge_issuer(id, accounts.charlie));

            // Can add code
            assert!(fat_badges
                .call_mut()
                .add_code(id, vec!["code1".to_string(), "code2".to_string()])
                .is_ok());
            stack.switch_account(accounts.bob).unwrap();
            assert_eq!(
                fat_badges.call_mut().add_code(id, vec![]),
                Err(Error::BadOrigin),
                "Only the badge owner can add code"
            );

            // Check the badge stats
            let badge = fat_badges.call().get_badge_info(id).unwrap();
            assert_eq!(badge.num_code, 2);
            assert_eq!(badge.num_issued, 0);

            // Can issue badges to Django and Eve
            stack.switch_account(accounts.alice).unwrap();
            assert!(fat_badges.call_mut().issue(id, accounts.django).is_ok());
            assert_eq!(
                fat_badges.call_mut().issue(id, accounts.django),
                Err(Error::Duplicated),
                "Cannot issue duplicated badges"
            );
            stack.switch_account(accounts.bob).unwrap();
            assert!(fat_badges.call_mut().issue(id, accounts.eve).is_ok());
            assert_eq!(
                fat_badges.call_mut().issue(id, accounts.frank),
                Err(Error::RunOutOfCode),
                "No code available to issue badges"
            );

            // Adding a new code solves the problem
            stack.switch_account(accounts.alice).unwrap();
            assert!(fat_badges
                .call_mut()
                .add_code(id, vec!["code3".to_string()])
                .is_ok());
            assert!(fat_badges.call_mut().issue(id, accounts.frank).is_ok());

            // Code can be revealed
            stack.switch_account(accounts.django).unwrap();
            assert_eq!(fat_badges.call().get(id), Ok("code1".to_string()));
            stack.switch_account(accounts.eve).unwrap();
            assert_eq!(fat_badges.call().get(id), Ok("code2".to_string()));
            stack.switch_account(accounts.frank).unwrap();
            assert_eq!(fat_badges.call().get(id), Ok("code3".to_string()));
            stack.switch_account(accounts.alice).unwrap();
            assert_eq!(fat_badges.call().get(id), Err(Error::NotFound));

            // Final checks
            let badge = fat_badges.call().get_badge_info(id).unwrap();
            assert_eq!(badge.num_code, 3);
            assert_eq!(badge.num_issued, 3);
            assert_eq!(fat_badges.call().get_total_badges(), 2);
        }
    }
}
