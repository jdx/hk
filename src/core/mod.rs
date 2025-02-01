use crate::plugins::plugin::*;
use indexmap::IndexMap;
use std::sync::LazyLock;

include!(concat!(env!("OUT_DIR"), "/core.rs"));

pub fn get(name: &str) -> &Plugin {
    CORES.get(name).unwrap()
}
