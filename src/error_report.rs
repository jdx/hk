//! eyre report formatting for errors that reach the terminal.
//!
//! eyre's default handler ends every error with a `Location:` trailer naming
//! the Rust source line that created it. That is useful to hk developers but
//! reads like a crash dump to users, so show it only when debug logging is on.

use std::error::Error as StdError;
use std::fmt;

use eyre::EyreHandler;

struct Handler {
    inner: Box<dyn EyreHandler>,
}

impl EyreHandler for Handler {
    fn debug(&self, error: &(dyn StdError + 'static), f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() || log::log_enabled!(log::Level::Debug) {
            return self.inner.debug(error, f);
        }
        write!(f, "{error}")?;
        if let Some(cause) = error.source() {
            write!(f, "\n\nCaused by:")?;
            let multiple = cause.source().is_some();
            let mut next = Some(cause);
            let mut n = 0;
            while let Some(error) = next {
                writeln!(f)?;
                let text = error.to_string();
                for (i, line) in text.lines().enumerate() {
                    if i > 0 {
                        writeln!(f)?;
                    }
                    if multiple && i == 0 {
                        write!(f, "{n: >4}: {line}")?;
                    } else if multiple {
                        write!(f, "      {line}")?;
                    } else {
                        write!(f, "    {line}")?;
                    }
                }
                next = error.source();
                n += 1;
            }
        }
        Ok(())
    }

    fn track_caller(&mut self, location: &'static std::panic::Location<'static>) {
        self.inner.track_caller(location);
    }
}

pub fn install() {
    let _ = eyre::set_hook(Box::new(|error| {
        Box::new(Handler {
            inner: eyre::DefaultHandler::default_with(error),
        })
    }));
}
