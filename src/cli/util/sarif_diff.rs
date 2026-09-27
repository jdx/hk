use crate::Result;
use indexmap::IndexMap;
use serde_json::Value;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Print a patch from the fixes in a tool's SARIF report, for a `check_diff` command
///
/// Runs COMMAND, reads the SARIF log it prints on stdout, and applies the
/// fixes its results carry to the files they name, printing a unified diff of
/// the changes. Exits 0 when there are no results, and 1 after printing the
/// patch when every result has a fix.
///
/// If any result has no fix, a fix can't be applied, or COMMAND fails without
/// reporting results, no patch is printed: the results or errors are shown and
/// hk runs the step's fixer instead, so an unfixable finding is never hidden
/// behind a patch that fixed the rest.
///
/// Example: `hk util sarif-diff -- pinact run --check --format sarif a.yml`
#[derive(Debug, usage_rs::Args)]
pub struct SarifDiff {
    /// The tool, printing a SARIF log on stdout
    #[usage(arg, required, double_dash = "required")]
    pub command: Vec<String>,
}

/// One replacement, as a byte range of the file and the text to put there.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Edit {
    start: usize,
    end: usize,
    text: String,
}

impl SarifDiff {
    pub async fn run(&self) -> Result<()> {
        let (program, args) = self.command.split_first().expect("command is required");
        let output = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|err| eyre::eyre!("{program}: {err}"))?;
        let status = output.status.code().filter(|&c| c != 0).unwrap_or(1);
        let fail = |message: &str| -> ! {
            let mut err = io::stderr().lock();
            let _ = err.write_all(&output.stderr);
            if !message.is_empty() {
                let _ = writeln!(err, "{message}");
            }
            std::process::exit(status);
        };

        let log: Value = match serde_json::from_slice(&output.stdout) {
            Ok(log) => log,
            Err(err) => fail(&format!("{program} did not print a SARIF log: {err}")),
        };
        let edits = match collect_edits(&log) {
            Ok(edits) => edits,
            Err(problems) => fail(&problems.join("\n")),
        };
        if edits.is_empty() {
            if output.status.success() {
                return Ok(());
            }
            fail("");
        }

        let mut patches = String::new();
        for (path, edits) in edits {
            let patch = match patch_file(&path, edits) {
                Ok(patch) => patch,
                Err(problem) => fail(&problem),
            };
            patches.push_str(&patch);
        }
        if patches.is_empty() {
            // Results whose fixes change nothing: let the fixer report them.
            fail("the SARIF fixes change no files");
        }
        let mut out = io::stdout().lock();
        out.write_all(patches.as_bytes())?;
        out.flush()?;
        std::process::exit(1);
    }
}

/// The edits for each file, in the order the files first appear, or a
/// description of each result that can't be turned into edits.
fn collect_edits(log: &Value) -> std::result::Result<IndexMap<PathBuf, Vec<Edit>>, Vec<String>> {
    let mut regions: IndexMap<PathBuf, Vec<(Region, String)>> = IndexMap::new();
    let mut problems = Vec::new();
    for run in array(&log["runs"]) {
        let utf16 = run["columnKind"].as_str() != Some("unicodeCodePoints");
        for result in array(&run["results"]) {
            // A result may offer alternative fixes; the first is applied.
            let Some(fix) = array(&result["fixes"]).first() else {
                problems.push(describe(result, "has no fix"));
                continue;
            };
            for change in array(&fix["artifactChanges"]) {
                let Some(path) = artifact_path(&change["artifactLocation"]) else {
                    problems.push(describe(result, "names no file"));
                    continue;
                };
                for replacement in array(&change["replacements"]) {
                    let Some(region) = Region::parse(&replacement["deletedRegion"], utf16) else {
                        problems.push(describe(result, "has a region hk can't read"));
                        continue;
                    };
                    let text = replacement["insertedContent"]["text"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string();
                    regions
                        .entry(path.clone())
                        .or_default()
                        .push((region, text));
                }
            }
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    let mut edits = IndexMap::new();
    for (path, regions) in regions {
        let content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(err) => return Err(vec![format!("{}: {err}", path.display())]),
        };
        let mut file_edits = Vec::with_capacity(regions.len());
        for (region, text) in regions {
            let Some((start, end)) = region.byte_range(&content) else {
                return Err(vec![format!(
                    "{}: a fix's region is outside the file",
                    path.display()
                )]);
            };
            file_edits.push(Edit { start, end, text });
        }
        edits.insert(path, file_edits);
    }
    Ok(edits)
}

fn array(value: &Value) -> &[Value] {
    value.as_array().map(Vec::as_slice).unwrap_or_default()
}

fn describe(result: &Value, problem: &str) -> String {
    let location = &result["locations"][0]["physicalLocation"];
    let file = location["artifactLocation"]["uri"].as_str().unwrap_or("?");
    let line = location["region"]["startLine"].as_u64().unwrap_or(0);
    let message = result["message"]["text"].as_str().unwrap_or_default();
    format!("{file}:{line}: {message} ({problem})")
}

/// The local path an `artifactLocation` names: a `file:` URI, or a path
/// relative to the working directory.
fn artifact_path(location: &Value) -> Option<PathBuf> {
    let uri = location["uri"].as_str()?;
    if uri.starts_with("file:") {
        return url::Url::parse(uri).ok()?.to_file_path().ok();
    }
    Some(PathBuf::from(percent_decode(uri)))
}

/// Decode `%XX` escapes in a relative URI reference.
fn percent_decode(uri: &str) -> String {
    let bytes = uri.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                i += 3;
            }
            (byte, _) => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A SARIF region, in characters of the run's column kind.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Region {
    Lines {
        start_line: usize,
        start_column: usize,
        end_line: usize,
        /// `None` for the end of `end_line`, before its line terminator.
        end_column: Option<usize>,
        utf16: bool,
    },
    Chars {
        offset: usize,
        length: usize,
        utf16: bool,
    },
}

impl Region {
    fn parse(region: &Value, utf16: bool) -> Option<Self> {
        let get = |key: &str| region[key].as_u64().map(|n| n as usize);
        if let Some(start_line) = get("startLine") {
            return Some(Region::Lines {
                start_line,
                start_column: get("startColumn").unwrap_or(1),
                end_line: get("endLine").unwrap_or(start_line),
                end_column: get("endColumn"),
                utf16,
            });
        }
        Some(Region::Chars {
            offset: get("charOffset")?,
            length: get("charLength").unwrap_or(0),
            utf16,
        })
    }

    fn byte_range(&self, content: &str) -> Option<(usize, usize)> {
        match *self {
            Region::Lines {
                start_line,
                start_column,
                end_line,
                end_column,
                utf16,
            } => {
                let (start_text, start_offset) = line(content, start_line)?;
                let start = start_offset + column_offset(start_text, start_column, utf16)?;
                let (end_text, end_offset) = line(content, end_line)?;
                let end = match end_column {
                    Some(column) => end_offset + column_offset(end_text, column, utf16)?,
                    None => end_offset + end_text.len(),
                };
                (start <= end).then_some((start, end))
            }
            Region::Chars {
                offset,
                length,
                utf16,
            } => {
                let start = char_offset(content, offset, utf16)?;
                let end = start + char_offset(&content[start..], length, utf16)?;
                Some((start, end))
            }
        }
    }
}

/// The text of 1-based line `n` without its terminator, and its byte offset.
fn line(content: &str, n: usize) -> Option<(&str, usize)> {
    let mut offset = 0;
    for (i, line) in content.split_inclusive('\n').enumerate() {
        if i + 1 == n {
            let text = line.strip_suffix('\n').unwrap_or(line);
            let text = text.strip_suffix('\r').unwrap_or(text);
            return Some((text, offset));
        }
        offset += line.len();
    }
    None
}

/// The byte offset of 1-based `column` in `text`, which may be one past its end.
fn column_offset(text: &str, column: usize, utf16: bool) -> Option<usize> {
    char_offset(text, column.checked_sub(1)?, utf16)
}

/// The byte offset after `count` characters of `text`, counted in UTF-16 code
/// units or in code points.
fn char_offset(text: &str, count: usize, utf16: bool) -> Option<usize> {
    let mut units = 0;
    for (offset, c) in text.char_indices() {
        if units == count {
            return Some(offset);
        }
        units += if utf16 { c.len_utf16() } else { 1 };
        if units > count {
            return None; // inside a surrogate pair
        }
    }
    (units == count).then_some(text.len())
}

/// The unified diff for applying `edits` to the file at `path`.
fn patch_file(path: &PathBuf, mut edits: Vec<Edit>) -> std::result::Result<String, String> {
    let display = path.display();
    let original = std::fs::read_to_string(path).map_err(|err| format!("{display}: {err}"))?;
    edits.sort_by_key(|edit| (edit.start, edit.end));
    // Tools can report the same fix for several results.
    edits.dedup();
    for pair in edits.windows(2) {
        if pair[1].start < pair[0].end
            || (pair[1].start == pair[0].start && pair[0].end == pair[0].start)
        {
            return Err(format!("{display}: two fixes change the same text"));
        }
    }
    let mut fixed = original.clone();
    for edit in edits.iter().rev() {
        fixed.replace_range(edit.start..edit.end, &edit.text);
    }
    if fixed == original {
        return Ok(String::new());
    }
    let label = display.to_string();
    Ok(crate::diff::render_unified_diff(
        &original, &fixed, &label, &label,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_whole_line_region_stops_before_the_line_terminator() {
        let region = Region::parse(&serde_json::json!({"startLine": 2}), true).unwrap();
        assert_eq!(region.byte_range("one\r\ntwo\r\nthree\n"), Some((5, 8)));
    }

    #[test]
    fn columns_count_utf16_code_units_by_default() {
        let content = "a😀b\n";
        let region = Region::parse(
            &serde_json::json!({"startLine": 1, "startColumn": 4, "endColumn": 5}),
            true,
        )
        .unwrap();
        assert_eq!(region.byte_range(content), Some((5, 6)));
        let region = Region::parse(
            &serde_json::json!({"startLine": 1, "startColumn": 3, "endColumn": 4}),
            false,
        )
        .unwrap();
        assert_eq!(region.byte_range(content), Some((5, 6)));
    }

    #[test]
    fn char_offsets_are_supported() {
        let region =
            Region::parse(&serde_json::json!({"charOffset": 4, "charLength": 3}), true).unwrap();
        assert_eq!(region.byte_range("one two\n"), Some((4, 7)));
    }

    #[test]
    fn regions_outside_the_file_are_rejected() {
        let region = Region::parse(&serde_json::json!({"startLine": 3}), true).unwrap();
        assert_eq!(region.byte_range("one\n"), None);
    }

    #[test]
    fn a_result_without_a_fix_is_a_problem() {
        let log = serde_json::json!({"runs": [{"results": [{
            "message": {"text": "bad"},
            "locations": [{"physicalLocation": {"artifactLocation": {"uri": "a.yml"}, "region": {"startLine": 2}}}]
        }]}]});
        assert_eq!(
            collect_edits(&log).unwrap_err(),
            vec!["a.yml:2: bad (has no fix)".to_string()]
        );
    }

    #[test]
    fn relative_and_file_uris_resolve_to_paths() {
        assert_eq!(
            artifact_path(&serde_json::json!({"uri": ".github/workflows/a%20b.yml"})),
            Some(PathBuf::from(".github/workflows/a b.yml"))
        );
        #[cfg(unix)]
        assert_eq!(
            artifact_path(&serde_json::json!({"uri": "file:///repo/a.yml"})),
            Some(PathBuf::from("/repo/a.yml"))
        );
    }
}
