local function table_empty(tbl)
    return next(tbl) == nil
end

local function table_size(tbl)
    local size = 0
    for _ in pairs(tbl) do
        size = size + 1
    end
    return size
end

local function dump(tbl, indent)
    indent = indent or 0
    if type(tbl) ~= "table" then
        print(tbl)
    else
        for k, v in pairs(tbl) do
            if type(v) == "table" then
                if table_empty(v) then
                    print(string.format("%s%s: {}", string.rep(" ", indent), k))
                else
                    print(string.format("%s%s: {", string.rep(" ", indent), k))
                    dump(v, indent + 2)
                    print(string.format("%s}", string.rep(" ", indent)))
                end
            elseif type(v) == "string" then
                print(string.format("%s%s: %q", string.rep(" ", indent), k, v))
            elseif type(v) == "nil" then
                print(string.format("%s%s: %s", string.rep(" ", indent), k, "nil"))
            elseif type(v) == "number" then
                print(string.format("%s%s: %s", string.rep(" ", indent), k, tostring(v)))
            elseif type(v) == "boolean" then
                print(string.format("%s%s: %s", string.rep(" ", indent), k, tostring(v)))
            else
                print(string.format("%s%s: %s", string.rep(" ", indent), k, tostring(v)))
            end
        end
    end
end

return {
    dump = dump,
    table_empty = table_empty,
    table_size = table_size,
}
