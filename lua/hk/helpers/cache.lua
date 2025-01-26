local next_key = 0
local M = {}

M._reset = function()
    M.cache = {}
end

M._reset()

---@class NullLsCacheParams
---@field bufnr number
---@field root string

--- creates a function that caches the output of a callback, indexed by bufnr
---@param cb function
---@return fun(params: NullLsCacheParams): any
function M.by_bufnr(cb)
    local key = next_key
    next_key = next_key + 1

    return function(params)
        local bufnr = params.bufnr
        if M.cache[key] == nil then
            M.cache[key] = {}
        end
        if M.cache[key][bufnr] == nil then
            M.cache[key][bufnr] = cb(params) or false
        end
        return M.cache[key][bufnr]
    end
end

return M
