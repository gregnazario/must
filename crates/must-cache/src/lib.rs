//! Caching layer with hash and mtime strategies, backed by sled.

pub mod hash;
pub mod mtime;
pub mod store;

pub use store::DiskCache;
