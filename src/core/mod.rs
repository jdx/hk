use std::{collections::BTreeMap, sync::LazyLock};

use crate::plugins::plugin::Plugin;

mod end_of_file_fixer;
pub mod prettier;

pub static CORE_PLUGINS: LazyLock<BTreeMap<&'static str, Box<dyn Plugin>>> = LazyLock::new(|| {
    let end_of_file_fixer = Box::new(end_of_file_fixer::EndOfFileFixer::default()) as Box<dyn Plugin>;
    BTreeMap::from_iter([
        (end_of_file_fixer.name(), end_of_file_fixer),
    ])
});
