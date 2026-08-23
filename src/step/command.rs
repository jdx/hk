use ensembler::CmdLineRunner;
use std::path::Path;

pub(crate) fn argv_runner(
    argv: &[String],
    cwd: &Path,
    path: Option<&str>,
    pathext: Option<&str>,
) -> CmdLineRunner {
    #[cfg(windows)]
    if let Some(program) = resolve_batch_file(&argv[0], cwd, path, pathext) {
        return batch_runner(&program, &argv[1..]);
    }

    let _ = (cwd, path, pathext);
    CmdLineRunner::new_direct(&argv[0]).args(&argv[1..])
}

#[cfg_attr(not(windows), allow(dead_code))]
fn resolve_batch_file(
    program: &str,
    cwd: &Path,
    path: Option<&str>,
    pathext: Option<&str>,
) -> Option<std::path::PathBuf> {
    use std::ffi::OsStr;
    use std::path::{MAIN_SEPARATOR, PathBuf};

    let program_path = Path::new(program);
    let has_dir =
        program_path.is_absolute() || program.contains('/') || program.contains(MAIN_SEPARATOR);
    let dirs: Vec<PathBuf> = if has_dir {
        vec![cwd.to_path_buf()]
    } else {
        std::iter::once(cwd.to_path_buf())
            .chain(std::env::split_paths(
                &path
                    .map(OsStr::new)
                    .map(ToOwned::to_owned)
                    .or_else(|| std::env::var_os("PATH"))
                    .unwrap_or_default(),
            ))
            .map(|dir| {
                if dir.is_absolute() {
                    dir
                } else {
                    cwd.join(dir)
                }
            })
            .collect()
    };

    if program_path.extension().is_some() {
        let candidate = if program_path.is_absolute() {
            program_path.to_path_buf()
        } else if has_dir {
            cwd.join(program_path)
        } else {
            dirs.iter()
                .map(|dir| dir.join(program_path))
                .find(|candidate| candidate.is_file())?
        };
        return is_batch_file(&candidate).then_some(candidate);
    }

    let extensions = pathext
        .map(str::to_owned)
        .or_else(|| std::env::var("PATHEXT").ok())
        .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".to_string());
    for dir in dirs {
        let base = if program_path.is_absolute() {
            program_path.to_path_buf()
        } else {
            dir.join(program_path)
        };
        for extension in extensions
            .split(';')
            .filter(|extension| !extension.is_empty())
        {
            let candidate = PathBuf::from(format!("{}{}", base.display(), extension));
            if candidate.is_file() {
                return is_batch_file(&candidate).then_some(candidate);
            }
        }
    }
    None
}

#[cfg_attr(not(windows), allow(dead_code))]
fn is_batch_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("cmd") || extension.eq_ignore_ascii_case("bat")
        })
}

#[cfg_attr(not(windows), allow(dead_code))]
fn batch_runner(program: &Path, args: &[String]) -> CmdLineRunner {
    let program = program.to_string_lossy().replace('/', "\\");
    let normalized = program.to_ascii_lowercase();
    // npm shims forward `%*` through a second cmd.exe parse, so metacharacters
    // need one additional escape pass to arrive as literal argv values.
    let double_escape = normalized.contains("\\node_modules\\.bin\\");
    let mut command = escape_cmd_meta(&program);
    for arg in args {
        command.push(' ');
        command.push_str(&escape_cmd_arg(arg, double_escape));
    }
    CmdLineRunner::new_direct(std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into()))
        .args(["/d", "/s", "/c"])
        .raw_arg(format!("\"{command}\""))
}

#[cfg_attr(not(windows), allow(dead_code))]
fn escape_cmd_arg(arg: &str, double_escape_meta: bool) -> String {
    // Match the quoting strategy used by cross-spawn: first apply Windows argv
    // backslash/quote rules, then protect cmd.exe metacharacters with carets.
    let mut escaped = String::with_capacity(arg.len() + 2);
    escaped.push('"');
    let mut backslashes = 0;
    for character in arg.chars() {
        if character == '\\' {
            backslashes += 1;
            continue;
        }
        if character == '"' {
            escaped.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
        } else {
            escaped.extend(std::iter::repeat_n('\\', backslashes));
        }
        backslashes = 0;
        escaped.push(character);
    }
    escaped.extend(std::iter::repeat_n('\\', backslashes * 2));
    escaped.push('"');
    let escaped = escape_cmd_meta(&escaped);
    if double_escape_meta {
        escape_cmd_meta(&escaped)
    } else {
        escaped
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
fn escape_cmd_meta(value: &str) -> String {
    const META: &str = "()[]%!^\"`<>&|;, *?";
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if META.contains(character) {
            escaped.push('^');
        }
        escaped.push(character);
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_cmd_shim_arguments() {
        assert_eq!(escape_cmd_arg("space value", false), "^\"space^ value^\"");
        assert_eq!(
            escape_cmd_arg("amp&percent%caret^", true),
            "^^^\"amp^^^&percent^^^%caret^^^^^^^\""
        );
    }
}
