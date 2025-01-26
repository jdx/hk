local debug = require("hk.debug")
local util = require("hk.utils.tbl_flatten")
local M = {}

---@class PathUtils
---@field exists fun(filename: string): boolean
---@field join fun(...: string): string
---@field ancestors fun(start_path: string): fun(): string
M.path = {
    exists = function(filename)
        local stat = uv.fs_stat(filename)
        return stat ~= nil
    end,
    exists_async = function(filename)
        local co = coroutine.running()
        local fs_stat_err = nil
        local fs_stat_stat = nil
        uv.fs_stat(filename, function(err, stat)
            fs_stat_err, fs_stat_stat = err, stat
            if coroutine.status(co) == "suspended" then
                coroutine.resume(co)
            end
        end)
        if fs_stat_err == nil and fs_stat_stat == nil then
            coroutine.yield()
        end
        return fs_stat_stat ~= nil
    end,
    join = function(...)
        local v, _ = table.concat(util.tbl_flatten({ ... }), path_separator):gsub(path_separator .. "+", path_separator)
        return v
    end,
    -- An iterator like vim.fs.parents but includes the start_path.
    ancestors = function(start_path)
        print(start_path)
        local function internal()
            coroutine.yield(start_path)
            for path in vim.fs.parents(start_path) do
                coroutine.yield(path)
            end
        end
        return coroutine.wrap(internal)
    end,
}

--- creates a callback that returns the first root matching a specified pattern
---@param ... string patterns
---@return fun(startpath: string): string|nil root_dir
M.root_pattern = function(...)
    local patterns = util.tbl_flatten({ ... })

    local function matcher(path)
        if not path then
            return nil
        end

        -- escape wildcard characters in the path so that it is not treated like a glob
        path = path:gsub("([%[%]%?%*])", "\\%1")
        for _, pattern in ipairs(patterns) do
            for _, p in ipairs(vim.fn.glob(M.path.join(path, pattern), true, true)) do
                if M.path.exists(p) then
                    return path
                end
            end
        end

        return nil
    end

    return function(start_path)
        for path in M.path.ancestors(start_path) do
            local match = matcher(path)
            if match then
                return match
            end
        end
    end
end

---@module "null-ls.utils.make_params"
M.make_params = function(...)
    return require("null-ls.utils.make_params")(...)
end

---@module "hk.utils.cosmiconfig"
M.cosmiconfig = function(...)
    return require("hk.utils.cosmiconfig")(...)
end

return M
