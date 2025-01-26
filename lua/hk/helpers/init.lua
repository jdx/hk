local debug = require("hk.debug")
local cache = require("hk.helpers.cache")

return {
    make_builtin = function(builtin)
        debug.dump(builtin)
        debug.dump(builtin.generator_opts.cwd({
            bufnr = 1,
            root = "/Users/jason/Projects/null-ls",
            bufname = "/Users/jason/Projects/null-ls/lua/hk/core/prettier.lua",
        }))
        return builtin
    end,
    range_formatting_args_factory = function(args, ...)
        return {
            args = args,
        }
    end,
    cache = cache,
}
