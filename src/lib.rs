//! Saena is a desk. One tree. Claim work. Complete a deed. Remember.

pub mod atom;
pub mod cards;
pub mod cli;
pub mod desk;
pub mod error;
pub mod extract;
pub mod id;
pub mod init;
pub mod install;
pub mod mcp;
pub mod node;
pub mod panel;
pub mod project;
pub mod search;
pub mod sit;
pub mod stamp;
pub mod store;

pub use desk::Desk;
pub use error::{Error, Result};
pub use node::{Kind, Node, Status};
