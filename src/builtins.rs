// Include generated builtins data
include!(concat!(env!("OUT_DIR"), "/builtins.rs"));

/// The mise tools that install what a step runs, when the step is a builtin.
///
/// A config names a builtin step after it (`["ruff"] = Builtins.ruff`), so
/// the step name is looked up first, then the missing `program` when argv
/// names it. Returns nothing for a step that isn't a builtin.
pub fn mise_tools_for_step(step_name: &str, program: Option<&str>) -> &'static [&'static str] {
    let find = |name: &str| {
        let name = name.replace('-', "_");
        BUILTINS_META
            .iter()
            .find(|meta| meta.name == name && !meta.mise_tools.is_empty())
    };
    find(step_name)
        .or_else(|| program.and_then(find))
        .map_or(&[], |meta| meta.mise_tools)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_tools_come_from_the_builtin() {
        assert_eq!(mise_tools_for_step("ruff", None), ["ruff"]);
        assert_eq!(mise_tools_for_step("lint", Some("ruff")), ["ruff"]);
        assert_eq!(
            mise_tools_for_step("editorconfig-checker", None),
            ["editorconfig-checker"]
        );
        assert!(mise_tools_for_step("custom", Some("custom-tool")).is_empty());
    }
}
