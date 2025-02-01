use std::{env, fs, path::Path};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

include!("src/plugins/plugin.rs");

// macro_rules! warn {
//     ($($arg:tt)*) => {
//         println!("cargo:warning={}", format!($($arg)*));
//     };
// }

pub fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    load_cores(&out_dir).unwrap();
}

fn load_cores(out_dir: &str) -> Result<()> {
    println!("cargo:rerun-if-changed=src/core");
    let cores = cores()?;
    let out_path = Path::new(&out_dir).join("core.rs");
    let mut out = vec![];
    out.push(r#"pub static CORES: LazyLock<IndexMap<&'static str, Plugin>> = LazyLock::new(|| {
        let mut cores = IndexMap::new();
"#.to_string());
    for core in &cores {
        let name = &core.name;
        out.push(format!(r#"        cores.insert("{name}", {name}());"#));
    }
    out.push(r#"        cores"#.to_string());
    out.push(r#"    });"#.to_string());
    for core in cores {
        let name = core.name;
        let command = core.format.command;
        let args = core.format.args;
        let description = core.meta.description;
        let url = core.meta.url;
        let to_stdin = core.format.to_stdin;
        out.push(format!(r#"
fn {name}() -> Plugin {{
    Plugin {{
        name: "{name}".to_string(),
        file_types: vec![],
        format: PluginFormat {{
            command: "{command}".to_string(),
            args: {args:?}.into_iter().map(|s| s.to_string()).collect(),
            to_stdin: {to_stdin},
        }},
        meta: PluginMeta {{
            description: "{description}".to_string(),
            url: "{url}".to_string(),
            notes: vec![],
        }},
    }}
}}
"#));
    }
    fs::write(&out_path, out.join("\n") + "\n")?;
    Ok(())
}

fn cores() -> Result<Vec<Plugin>> {
    let mut cores = Vec::new();
    for entry in fs::read_dir("src/core").unwrap() {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.to_str().unwrap().ends_with(".pkl") {
            let plugin: Plugin = rpkl::from_config(&path)?;
            cores.push(plugin);
        }
    }
    Ok(cores)
}
