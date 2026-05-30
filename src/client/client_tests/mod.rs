// Client integration tests
//
// Each module is a separate file to keep under 350 lines.
// The unit module provides shared test infrastructure (shared_client, run_may).

mod integration_hashes_advanced;
mod integration_hashes_basic;
mod integration_lists_basic;
mod integration_sets_basic;
mod integration_sorted_sets;

pub mod unit;
