// Minimal tests module for the `party_mode` module.
//
// This file exists to satisfy the compiler error when `#[cfg(test)] mod tests;`
// is present in the parent module but the file is missing.
//
// Keep tests tiny and independent so they don't impose requirements on the module.

#[test]
fn smoke_test() {
  // simple no-op test to ensure test module compiles and runs
  assert!(true);
}
