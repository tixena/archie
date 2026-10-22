//! Core library for archie: markdown documents as a typed tree.

extern crate alloc;

tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

pub mod bundle;
pub mod forms;
pub mod parser;
pub mod value;
pub mod writer;
