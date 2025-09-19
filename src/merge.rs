#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HunkSource {
    Fixer,
    Worktree,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    pub start: usize, // inclusive line index in base
    pub end: usize,   // exclusive line index in base
    pub lines: Vec<String>,
    pub source: HunkSource,
}

/// Compute line-based diff hunks of `other` relative to `base` using LCS.
pub fn diff_hunks(base: &str, other: &str, source: HunkSource) -> Vec<Hunk> {
    if base == other {
        return vec![];
    }
    let a: Vec<&str> = base.split_inclusive('\n').collect();
    let b: Vec<&str> = other.split_inclusive('\n').collect();
    let n = a.len();
    let m = b.len();
    // LCS DP
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    // Walk to build change regions
    let mut i = 0usize;
    let mut j = 0usize;
    let mut hunks: Vec<Hunk> = vec![];
    let mut cur_start: Option<usize> = None;
    let mut cur_lines: Vec<String> = vec![];
    while i < n && j < m {
        if a[i] == b[j] {
            if let Some(start) = cur_start.take() {
                hunks.push(Hunk {
                    start,
                    end: i,
                    lines: cur_lines.clone(),
                    source,
                });
                cur_lines.clear();
            }
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            // deletion in other -> part of changed region
            if cur_start.is_none() {
                cur_start = Some(i);
            }
            i += 1;
        } else {
            // insertion from other
            if cur_start.is_none() {
                cur_start = Some(i);
            }
            cur_lines.push(b[j].to_string());
            j += 1;
        }
    }
    if let Some(start) = cur_start.take() {
        // consume remaining insertions
        while j < m {
            cur_lines.push(b[j].to_string());
            j += 1;
        }
        hunks.push(Hunk {
            start,
            end: i,
            lines: cur_lines,
            source,
        });
    } else if j < m {
        // pure tail insertion at end
        hunks.push(Hunk {
            start: n,
            end: n,
            lines: b[j..].iter().map(|s| (*s).to_string()).collect(),
            source,
        });
    }
    hunks
}

/// Merge fixer and worktree hunks with preference to Worktree on overlap.
pub fn three_way_merge_hunks(base: &str, fixer: Option<&str>, worktree: Option<&str>) -> String {
    match (fixer, worktree) {
        (None, None) => base.to_string(),
        (Some(f), None) => f.to_string(),
        (None, Some(w)) => w.to_string(),
        (Some(f), Some(w)) => {
            let a: Vec<&str> = base.split_inclusive('\n').collect();
            let mut result: Vec<String> = Vec::new();
            let mut idx = 0usize;
            let mut fi = 0usize;
            let mut wi = 0usize;
            let fixer_hunks = diff_hunks(base, f, HunkSource::Fixer);
            let work_hunks = diff_hunks(base, w, HunkSource::Worktree);

            while fi < fixer_hunks.len() || wi < work_hunks.len() {
                let fh = fixer_hunks.get(fi);
                let wh = work_hunks.get(wi);

                // Choose next hunk to apply; if overlapping, prefer worktree
                let take_worktree = match (fh, wh) {
                    (Some(fh), Some(wh)) => {
                        wh.start < fh.end && fh.start < wh.end || wh.start <= fh.start
                    }
                    (None, Some(_)) => true,
                    (Some(_), None) => false,
                    (None, None) => false,
                };
                let (start, end, lines) = if take_worktree {
                    let h = wh.unwrap();
                    (h.start, h.end, h.lines.clone())
                } else {
                    let h = fh.unwrap();
                    (h.start, h.end, h.lines.clone())
                };

                // Append unchanged region up to start
                if idx < start {
                    result.extend(a[idx..start].iter().map(|s| (*s).to_string()));
                }
                // Apply chosen hunk
                result.extend(lines);
                idx = end;

                // Advance consumed hunk indices. If overlapping, skip any hunks fully covered by idx.
                if take_worktree {
                    wi += 1;
                } else {
                    fi += 1;
                }
                // Skip any hunks that begin before the current position to avoid partial re-application
                while fi < fixer_hunks.len() && fixer_hunks[fi].start < idx {
                    fi += 1;
                }
                while wi < work_hunks.len() && work_hunks[wi].start < idx {
                    wi += 1;
                }
            }
            // Tail unchanged
            if idx < a.len() {
                result.extend(a[idx..].iter().map(|s| (*s).to_string()));
            }
            result.concat()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{HunkSource, diff_hunks, three_way_merge_hunks};

    #[test]
    fn prefer_worktree_when_conflict() {
        let base = "fn f() { 1 }\n";
        let fixer = Some("fn f() { 1; }\n");
        let work = Some("fn f(){return 2}\n");
        let merged = three_way_merge_hunks(base, fixer, work);
        assert!(merged.contains("return 2"));
    }

    #[test]
    fn take_fixer_when_no_worktree_change() {
        let base = "a\n";
        let fixer = Some("a;\n");
        let work = Some("a\n");
        let merged = three_way_merge_hunks(base, fixer, work);
        assert_eq!(merged, "a;\n");
    }

    #[test]
    fn diff_produces_hunks() {
        let base = "a\nb\nc\n";
        let other = "a\nB\nc\nd\n";
        let hunks = diff_hunks(base, other, HunkSource::Fixer);
        assert_eq!(hunks.len(), 2);
        assert_eq!(hunks[0].start, 1);
        assert!(hunks[0].lines.join("").contains("B"));
    }
}
