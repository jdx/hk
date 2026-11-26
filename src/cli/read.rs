use crate::Result;

/// Serve hk's Pkl modules to pkl as an external module reader
///
/// pkl starts this command to read `hk:` modules, so a config can
/// `amends "hk:/Config.pkl"` and `import "hk:/Builtins.pkl"` and get the
/// modules of the installed hk instead of a pinned package. It speaks Pkl's
/// external reader protocol on stdin and stdout and is not meant to be run
/// by hand. Register it in a PklProject:
///
///     evaluatorSettings {
///       externalModuleReaders {
///         ["hk"] { executable = "hk"; arguments { "read" } }
///       }
///     }
///
/// or pass `--external-module-reader hk='hk read'` to `pkl eval`.
#[derive(Debug, usage_rs::Args)]
#[usage(effect = "read")]
pub struct Read;

impl Read {
    pub async fn run(&self) -> Result<()> {
        tokio::task::spawn_blocking(|| {
            let mut input = std::io::stdin().lock();
            let mut output = std::io::BufWriter::new(std::io::stdout().lock());
            crate::pkl_modules::serve(&mut input, &mut output)
        })
        .await??;
        Ok(())
    }
}
