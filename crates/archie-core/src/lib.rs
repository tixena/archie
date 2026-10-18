//! Core library for archie: markdown documents as a typed tree.

tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

pub mod forms;
pub mod parser;
pub mod writer;
