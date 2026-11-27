//! hk's own Pkl modules, served under the `hk:` URI scheme.
//!
//! A config can `amends "hk:/Config.pkl"` and `import "hk:/Builtins.pkl"`
//! instead of pinning a versioned `package://` URL: the modules come from the
//! hk that evaluates the config. hk serves them to its own evaluator directly,
//! and to the `pkl` CLI through `hk read`, which speaks Pkl's external
//! reader protocol.
//!
//! The URIs are hierarchical (`hk:/builtins/prettier.pkl`) because the
//! modules import each other with relative URIs, which Pkl cannot resolve
//! against an opaque URI such as `hk:Config`.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::sync::LazyLock;

use pklr::external_reader::{Message, ModuleReaderSpec, read_message, write_message};

/// The URI scheme hk's modules are served under.
pub const SCHEME: &str = "hk";

/// The pkl package for this version, staged by `build/embed_pkl_package.rs`.
/// Empty when the pkl sources were not generated.
pub static EMBEDDED_PKL_PACKAGE: &[u8] =
    include_bytes!(concat!(std::env!("OUT_DIR"), "/hk_pkl_package.zip"));

/// The root that generated configs import hk's modules from: `hk:` when this
/// hk carries its modules, or else the package URL of this hk's release, which
/// hk downloads.
pub fn import_root() -> String {
    if EMBEDDED_PKL_PACKAGE.is_empty() {
        let version = crate::version::version();
        format!("package://github.com/jdx/hk/releases/download/v{version}/hk@{version}#")
    } else {
        format!("{SCHEME}:")
    }
}

/// The modules of the embedded package, by their path in it
/// (`builtins/prettier.pkl`), or why they are unavailable.
static MODULES: LazyLock<Result<HashMap<String, String>, String>> =
    LazyLock::new(|| unpack(EMBEDDED_PKL_PACKAGE));

fn unpack(archive: &[u8]) -> Result<HashMap<String, String>, String> {
    if archive.is_empty() {
        return Err(
            "this hk was built without its Pkl modules; run `mise run pkl:gen` and rebuild"
                .to_string(),
        );
    }
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(archive))
        .map_err(|e| format!("hk's embedded Pkl package is unreadable: {e}"))?;
    let mut modules = HashMap::new();
    for index in 0..zip.len() {
        let mut file = zip
            .by_index(index)
            .map_err(|e| format!("hk's embedded Pkl package is unreadable: {e}"))?;
        let name = file
            .name()
            .map_err(|e| format!("hk's embedded Pkl package is unreadable: {e}"))?
            .to_string();
        if !file.is_file() || !name.ends_with(".pkl") {
            continue;
        }
        let mut source = String::new();
        std::io::Read::read_to_string(&mut file, &mut source)
            .map_err(|e| format!("hk's embedded {name} is unreadable: {e}"))?;
        modules.insert(name, source);
    }
    Ok(modules)
}

/// The source of the module at `uri` (`hk:/Config.pkl`), or `None` when hk
/// has no module there. Errors when hk was built without its modules.
pub fn read_module(uri: &str) -> Result<Option<String>, String> {
    let modules = MODULES.as_ref().map_err(Clone::clone)?;
    let Some(path) = uri
        .strip_prefix(SCHEME)
        .and_then(|rest| rest.strip_prefix(":/"))
    else {
        return Ok(None);
    };
    Ok(modules.get(path).cloned())
}

/// The module reader hk's own evaluator uses for `hk:` imports.
pub fn pklr_reader(uri: &str) -> pklr::Result<Option<String>> {
    read_module(uri).map_err(pklr::Error::Eval)
}

/// Answer Pkl's external reader requests on `input` until the evaluator asks
/// the process to close or closes its end. This is `hk read`.
pub fn serve(input: &mut impl BufRead, output: &mut impl Write) -> std::io::Result<()> {
    while let Some(message) = read_message(input)? {
        let response = match message {
            Message::InitializeModuleReaderRequest { request_id, scheme } => {
                Message::InitializeModuleReaderResponse {
                    request_id,
                    spec: (scheme == SCHEME).then_some(ModuleReaderSpec {
                        scheme,
                        has_hierarchical_uris: true,
                        is_local: true,
                        is_globbable: false,
                    }),
                }
            }
            // hk serves modules only.
            Message::InitializeResourceReaderRequest { request_id, .. } => {
                Message::InitializeResourceReaderResponse {
                    request_id,
                    spec: None,
                }
            }
            Message::ReadModuleRequest {
                request_id,
                evaluator_id,
                uri,
            } => {
                let (contents, error) = match read_module(&uri) {
                    Ok(Some(source)) => (Some(source), None),
                    Ok(None) => (
                        None,
                        Some(format!(
                            "hk {} has no module at {uri}",
                            crate::version::version()
                        )),
                    ),
                    Err(error) => (None, Some(error)),
                };
                Message::ReadModuleResponse {
                    request_id,
                    evaluator_id,
                    contents,
                    error,
                }
            }
            Message::ReadResourceRequest {
                request_id,
                evaluator_id,
                uri,
            } => Message::ReadResourceResponse {
                request_id,
                evaluator_id,
                contents: None,
                error: Some(format!("hk does not serve resources: {uri}")),
            },
            Message::ListModulesRequest {
                request_id,
                evaluator_id,
                ..
            } => Message::ListModulesResponse {
                request_id,
                evaluator_id,
                path_elements: None,
                error: Some("hk modules cannot be globbed".to_string()),
            },
            Message::ListResourcesRequest {
                request_id,
                evaluator_id,
                ..
            } => Message::ListResourcesResponse {
                request_id,
                evaluator_id,
                path_elements: None,
                error: Some("hk does not serve resources".to_string()),
            },
            Message::CloseExternalProcess => return Ok(()),
            // Responses never come from the evaluator, and Pkl may add
            // messages a reader should ignore.
            _ => continue,
        };
        write_message(output, &response)?;
        output.flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exchange(requests: &[Message]) -> Vec<Message> {
        let mut input = Vec::new();
        for request in requests {
            write_message(&mut input, request).unwrap();
        }
        let mut output = Vec::new();
        serve(&mut std::io::Cursor::new(input), &mut output).unwrap();
        let mut responses = Vec::new();
        let mut output = std::io::Cursor::new(output);
        while let Some(message) = read_message(&mut output).unwrap() {
            responses.push(message);
        }
        responses
    }

    #[test]
    fn initializes_only_the_hk_scheme() {
        let responses = exchange(&[
            Message::InitializeModuleReaderRequest {
                request_id: 1,
                scheme: "hk".into(),
            },
            Message::InitializeModuleReaderRequest {
                request_id: 2,
                scheme: "other".into(),
            },
            Message::InitializeResourceReaderRequest {
                request_id: 3,
                scheme: "hk".into(),
            },
        ]);
        assert_eq!(
            responses,
            vec![
                Message::InitializeModuleReaderResponse {
                    request_id: 1,
                    spec: Some(ModuleReaderSpec {
                        scheme: "hk".into(),
                        has_hierarchical_uris: true,
                        is_local: true,
                        is_globbable: false,
                    }),
                },
                Message::InitializeModuleReaderResponse {
                    request_id: 2,
                    spec: None,
                },
                Message::InitializeResourceReaderResponse {
                    request_id: 3,
                    spec: None,
                },
            ]
        );
    }

    #[test]
    fn stops_at_close() {
        let responses = exchange(&[
            Message::CloseExternalProcess,
            Message::InitializeModuleReaderRequest {
                request_id: 1,
                scheme: "hk".into(),
            },
        ]);
        assert!(responses.is_empty());
    }

    #[test]
    fn unknown_modules_are_errors() {
        let responses = exchange(&[Message::ReadModuleRequest {
            request_id: 7,
            evaluator_id: 1,
            uri: "hk:/Nope.pkl".into(),
        }]);
        let [
            Message::ReadModuleResponse {
                request_id: 7,
                contents: None,
                error: Some(error),
                ..
            },
        ] = responses.as_slice()
        else {
            panic!("unexpected responses: {responses:?}");
        };
        assert!(
            error.contains("hk:/Nope.pkl") || error.contains("pkl:gen"),
            "{error}"
        );
    }

    #[test]
    fn serves_the_embedded_modules() {
        if EMBEDDED_PKL_PACKAGE.is_empty() {
            return;
        }
        let responses = exchange(&[
            Message::ReadModuleRequest {
                request_id: 1,
                evaluator_id: 1,
                uri: "hk:/Config.pkl".into(),
            },
            Message::ReadModuleRequest {
                request_id: 2,
                evaluator_id: 1,
                uri: "hk:/Builtins.pkl".into(),
            },
        ]);
        assert_eq!(responses.len(), 2);
        for response in responses {
            let Message::ReadModuleResponse {
                contents: Some(source),
                error: None,
                ..
            } = response
            else {
                panic!("unexpected response: {response:?}");
            };
            assert!(!source.is_empty());
        }
    }

    #[test]
    fn unpack_requires_the_package() {
        assert!(unpack(&[]).unwrap_err().contains("pkl:gen"));
    }
}
