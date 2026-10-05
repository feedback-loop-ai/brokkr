//! brokkr-core's integration tests, linked as one binary (#423): each
//! file under tests/ is a module here, so its tests run as
//! `<file>::<name>`, and brokkr-cli's `test_targets` refuses a file that
//! is not.

mod differential;
mod fold_test;
mod policy_lint;
mod realms;
