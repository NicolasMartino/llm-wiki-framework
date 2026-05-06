pub mod collision;
pub mod hash;
mod schema;

pub use schema::{
    BackupEntry, BinaryEntry, FileKind, HashAlgorithm, Manifest, ManifestEntry, Ownership,
    PartialInstall, RuntimeName,
};
