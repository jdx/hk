#[macro_use]
extern crate log;
mod cmd;
mod error;

pub use error::{Error, Result};
pub use cmd::CmdLineRunner;
