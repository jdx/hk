//! "Did you mean" suggestions for misspelled names.

/// Levenshtein distance between two strings, counted in characters.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur.push((prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// The candidate closest to `name`, if one is close enough to be a likely typo.
pub fn did_you_mean<'a, I>(name: &str, candidates: I) -> Option<&'a str>
where
    I: IntoIterator<Item = &'a str>,
{
    let lower = name.to_lowercase();
    let limit = (name.chars().count() / 3).max(1);
    candidates
        .into_iter()
        .filter(|c| *c != name)
        .map(|c| (edit_distance(&lower, &c.to_lowercase()), c))
        .filter(|(distance, _)| *distance <= limit)
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, c)| c)
}

/// A trailing ` Did you mean 'x'?` for an error or warning message, or an empty
/// string when nothing is close.
pub fn did_you_mean_hint<'a, I>(name: &str, candidates: I) -> String
where
    I: IntoIterator<Item = &'a str>,
{
    did_you_mean(name, candidates)
        .map(|c| format!(" Did you mean '{c}'?"))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggests_close_names_only() {
        let names = ["check", "fix", "prettier"];
        assert_eq!(did_you_mean("chek", names), Some("check"));
        assert_eq!(did_you_mean("Prettier", names), Some("prettier"));
        assert_eq!(did_you_mean("prettyer", names), Some("prettier"));
        assert_eq!(did_you_mean("trial", names), None);
        assert_eq!(did_you_mean("check", names), None);
        assert_eq!(did_you_mean("x", names), None);
    }
}
