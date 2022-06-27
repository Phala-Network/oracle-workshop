#![cfg_attr(not(feature = "std"), no_std)]

///! Fat Contract utilities
pub mod attestation;
#[cfg(feature = "std")]
pub mod test_helper;
