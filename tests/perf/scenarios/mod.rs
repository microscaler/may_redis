// Performance test scenarios.
//
// Each module implements one benchmark scenario from the perf test plan.

pub mod population;
pub mod login_burst;
pub mod token_refresh;
pub mod authz_load;
pub mod mixed_workload;
