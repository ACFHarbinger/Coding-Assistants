-- Coding-Assistants Aseprite dispatch script.
--
-- Run by `crates/mcp-aseprite` as:
--   aseprite -b --script dispatch.lua --script-param op=<tool> --script-param <k>=<v> ...
--
-- Does one file-oriented operation and prints exactly one JSON line:
--   {"ok":true,"result":<value>}   or   {"ok":false,"error":"<msg>"}
--
-- Aseprite bundles no JSON library, so requests arrive as flat
-- `--script-param` scalars (in `app.params`) and this script hand-rolls a
-- small JSON encoder for the reply.

local P = app.params or {}

-- ---------------------------------------------------------------- json out

local function esc(s)
  return (s:gsub('[%z\1-\31\\"]', function(c)
    local m = { ['"'] = '\\"', ['\\'] = '\\\\', ['\n'] = '\\n', ['\r'] = '\\r', ['\t'] = '\\t' }
    return m[c] or string.format('\\u%04x', c:byte())
  end))
end

local function enc(v)
  local t = type(v)
  if t == 'string' then
    return '"' .. esc(v) .. '"'
  elseif t == 'number' then
    return tostring(v)
  elseif t == 'boolean' then
    return tostring(v)
  elseif t == 'nil' then
    return 'null'
  elseif t == 'table' then
    if #v > 0 or next(v) == nil then
      local parts = {}
      for _, item in ipairs(v) do parts[#parts + 1] = enc(item) end
      return '[' .. table.concat(parts, ',') .. ']'
    end
    local parts = {}
    for k, item in pairs(v) do
      parts[#parts + 1] = '"' .. esc(tostring(k)) .. '":' .. enc(item)
    end
    return '{' .. table.concat(parts, ',') .. '}'
  end
  return '"' .. esc(tostring(v)) .. '"'
end

local function ok(result) print(enc({ ok = true, result = result })) end
local function fail(msg) print(enc({ ok = false, error = tostring(msg) })) end

-- ---------------------------------------------------------------- helpers

local function open_sprite(path)
  if not path or path == '' then error('missing "path" parameter') end
  local spr = app.open(path)
  if not spr then error('could not open sprite: ' .. path) end
  return spr
end

local function color_mode_name(m)
  if m == ColorMode.RGB then return 'rgb' end
  if m == ColorMode.GRAYSCALE then return 'grayscale' end
  if m == ColorMode.INDEXED then return 'indexed' end
  if m == ColorMode.TILEMAP then return 'tilemap' end
  return tostring(m)
end

local function hex(c)
  return string.format('#%02X%02X%02X%02X', c.red, c.green, c.blue, c.alpha)
end

-- ---------------------------------------------------------------- ops

local ops = {}

function ops.sprite_info()
  local spr = open_sprite(P.path)
  local pal = spr.palettes[1]
  local info = {
    path = P.path,
    width = spr.width,
    height = spr.height,
    color_mode = color_mode_name(spr.colorMode),
    frames = #spr.frames,
    layers = #spr.layers,
    palette_size = pal and #pal or 0,
  }
  spr:close()
  return info
end

function ops.list_layers()
  local spr = open_sprite(P.path)
  local out = {}
  for _, layer in ipairs(spr.layers) do
    out[#out + 1] = { name = layer.name, visible = layer.isVisible, group = layer.isGroup }
  end
  spr:close()
  return out
end

function ops.export()
  local spr = open_sprite(P.path)
  local scale = tonumber(P.scale) or 1
  if scale > 1 then spr:resize(spr.width * scale, spr.height * scale) end
  spr:saveCopyAs(P.out)
  spr:close()
  return { saved = P.out, scale = scale }
end

function ops.resize()
  local spr = open_sprite(P.path)
  local w = tonumber(P.width)
  local h = tonumber(P.height)
  if not w or not h then error('resize needs numeric "width" and "height"') end
  spr:resize(w, h)
  local dest = (P.out ~= nil and P.out ~= '') and P.out or P.path
  spr:saveCopyAs(dest)
  spr:close()
  return { saved = dest, width = w, height = h }
end

function ops.export_spritesheet()
  local spr = open_sprite(P.path)
  local params = {
    type = SpriteSheetType.PACKED,
    textureFilename = P.out,
    dataFilename = P.out .. '.json',
  }
  if P.columns and P.columns ~= '' then
    params.type = SpriteSheetType.ROWS
    params.columns = tonumber(P.columns)
  end
  app.command.ExportSpriteSheet(params)
  spr:close()
  return { sheet = P.out, data = P.out .. '.json' }
end

function ops.get_palette()
  local spr = open_sprite(P.path)
  local pal = spr.palettes[1]
  local out = {}
  if pal then
    for i = 0, #pal - 1 do out[#out + 1] = hex(pal:getColor(i)) end
  end
  spr:close()
  return out
end

function ops.apply_script()
  local spr = open_sprite(P.path)
  local env = setmetatable({ spr = spr, result = nil, no_save = false }, { __index = _G })
  local chunk, cerr = load(P.code or '', 'apply_script', 't', env)
  if not chunk then
    spr:close()
    error('Lua compile error: ' .. tostring(cerr))
  end
  chunk()
  if not env.no_save then spr:saveCopyAs(P.path) end
  local r = env.result
  spr:close()
  return { result = r ~= nil and tostring(r) or nil }
end

-- ---------------------------------------------------------------- run

local op = P.op
if not op or not ops[op] then
  fail('unknown op ' .. tostring(op))
  return
end

local success, res = pcall(ops[op])
if success then
  ok(res)
else
  fail(res)
end
