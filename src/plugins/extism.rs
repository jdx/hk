use crate::Result;

pub fn run(wasm: extism::Wasm) -> Result<()> {
    let manifest = extism::Manifest::new([wasm]);
    let mut plugin = extism::Plugin::new(&manifest, [], true).unwrap();
    let res = plugin
        .call::<&str, &str>("count_vowels", "Hello, world!")
        .unwrap();
    println!("{}", res);
    Ok(())
}
