//! The binary against the MongoDB server `just test-stores` starts, in a
//! database of its own, dropped when the test ends.

#[cfg(test)]
#[cfg(store_tests)]
#[cfg(feature = "mongodb")]
#[path = "mongo/tests.rs"]
mod tests;
