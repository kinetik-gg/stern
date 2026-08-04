//! Stern's parity conformance claims ledger.
//!
//! The data lives next to this crate: [`claims.json`](../claims.json) is the
//! hand-curated claim manifest and [`README.md`](../README.md) states the
//! claim rules. This crate exports nothing; it exists so the machine
//! validator in `tests/claims_contract.rs` runs as a workspace-native test
//! (`cargo test --workspace` and the explicit CI step) without adding
//! dependencies to any shipping crate.
