//! Controllers - background reconciliation logic
//!
//! Controllers watch for resource changes and reconcile state.

mod resolution;

pub use resolution::PlanResolver;
