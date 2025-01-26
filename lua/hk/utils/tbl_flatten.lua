local M = {}

--- flattens a table
---@param t table
---@return table
function M.tbl_flatten(t)
    local res = {}
    local function flatten(v)
        if type(v) ~= "table" then
            table.insert(res, v)
            return
        end
        for _, v in ipairs(v) do
            flatten(v)
        end
    end
    flatten(t)
    return res
end

return M
