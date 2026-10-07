pub mod collision;
pub mod manifest;
pub(crate) mod profile;
pub mod sources;
pub(crate) mod template;

mod answers;
pub mod blueprints;
mod command;
pub mod compose;
mod managed_block;
pub mod packs;
mod runtime;
mod scaffold;

pub use command::run;
