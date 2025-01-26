use crate::Result;

pub fn run() -> Result<()> {
    // let lua = Lua::new();
    // // lua.load_std_libs(mlua::StdLib::DEBUG)?;
    // let package: Table = lua.globals().get("package")?;
    // let loaded: Table = package.get("loaded")?;
    // let debug: Table = lua.load(include_str!("../../lua/hk/debug.lua")).set_name("lua/hk/debug.lua").eval()?;
    // loaded.set("hk.debug", debug)?;

    // // Create methods with internal
    // let internal = Internal {
    //     FORMATTING: "FORMATTING".to_string(),
    //     RANGE_FORMATTING: "RANGE_FORMATTING".to_string(),
    // };
    // let methods = Methods { internal };
    // let command_resolver = lua.create_table()?;
    // command_resolver.set("from_node_modules", lua.create_function(|_, args: MultiValue| {
    //     dbg!(args);
    //     Ok(())
    // })?)?;

    // // Set tables in loaded
    // let helpers_cache: Table = lua.load(include_str!("../../lua/hk/helpers/cache.lua")).set_name("lua/hk/helpers/cache.lua").eval()?;
    // loaded.set("hk.helpers.cache", helpers_cache)?;
    // let helpers: Table = lua.load(include_str!("../../lua/hk/helpers/init.lua")).set_name("lua/hk/helpers/init.lua").eval()?;
    // loaded.set("hk.helpers", helpers)?;
    // loaded.set("hk.helpers.command_resolver", command_resolver)?;
    // loaded.set("hk.methods", lua.to_value(&methods)?)?;
    // let utils_tbl_flatten: Table = lua.load(include_str!("../../lua/hk/utils/tbl_flatten.lua")).set_name("lua/hk/utils/tbl_flatten.lua").eval()?;
    // loaded.set("hk.utils.tbl_flatten", utils_tbl_flatten)?;
    // let utils: Table = lua.load(include_str!("../../lua/hk/utils/init.lua")).set_name("lua/hk/utils/init.lua").eval()?;
    // loaded.set("hk.utils", utils)?;
    // let utils_cosmiconfig: Function = lua.load(include_str!("../../lua/hk/utils/cosmiconfig.lua")).set_name("lua/hk/utils/cosmiconfig.lua").eval()?;
    // loaded.set("hk.utils.cosmiconfig", utils_cosmiconfig)?;
    // // Load and evaluate Lua code
    // lua.load(include_str!("../../lua/hk/core/prettier.lua")).set_name("lua/hk/core/prettier.lua").eval()?;
    Ok(())
}
