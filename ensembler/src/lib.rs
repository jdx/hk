#[macro_use]
extern crate log;
mod cmd;
mod error;

pub use cmd::{CmdLineRunner, CmdResult};
pub use error::{Error, Result};
