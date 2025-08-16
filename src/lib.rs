extern crate log;

pub use error::{Error, Result};

mod error;
pub mod progress;
mod progress_bar;
mod style;
mod tracing;

// Initialize tracing on module load
static _INIT: std::sync::Once = std::sync::Once::new();

fn init() {
    _INIT.call_once(|| {
        tracing::init();
    });
}
