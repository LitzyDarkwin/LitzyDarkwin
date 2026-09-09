-- GatherOptimizer/GatherMate.lua — OPTIONAL GatherMate2 adapter.
-- Feature-detected at call time; the addon works identically without GatherMate2.
-- We never depend on its internals: only a small, type-checked public surface.
local _, GO = ...
if GO == nil then GO = _G.__GO_TEST_NS or {} end

local Adapter = {}
GO.GatherMate = Adapter
GO.modules["GatherMate"] = Adapter

--- True when a GatherMate2 addon object is present (any known global shape).
function Adapter.IsAvailable()
  local gm = _G.GatherMate2 or _G.GatherMate
  return gm ~= nil
end

--- Capability probe: what can we actually do with the installed version?
function Adapter.Capabilities()
  local gm = _G.GatherMate2 or _G.GatherMate
  local caps = { available = gm ~= nil, nodeQuery = false, displayControl = false }
  if type(gm) ~= "table" then return caps end
  -- Node queries (names vary by GatherMate2 version; probe, don't assume).
  for _, key in ipairs({ "GetNodesForZone", "GetNodes", "GetNodeData", "GetZoneData" }) do
    if type(gm[key]) == "function" then caps.nodeQuery = true break end
  end
  -- Display-module toggles.
  local display = gm.Display and true or false
  if display or type(gm.ShowNodes) == "function" or type(gm.SetEnabled) == "function" then
    caps.displayControl = true
  end
  return caps
end

--- Best-effort node lookup for a zone. Always returns a table (possibly empty).
-- Shape: { { x=0-100, y=0-100, kind="herb"|"ore"|"fish"|"gas"|"treasure"|"unknown" }, ... }
function Adapter.GetZoneNodes(zoneName)
  local gm = _G.GatherMate2 or _G.GatherMate
  if type(gm) ~= "table" or type(zoneName) ~= "string" then return {} end
  local raw
  local ok = false
  if type(gm.GetNodesForZone) == "function" then
    ok, raw = pcall(gm.GetNodesForZone, gm, zoneName)
  elseif type(gm.GetNodes) == "function" then
    ok, raw = pcall(gm.GetNodes, gm, zoneName)
  elseif type(gm.GetNodeData) == "function" then
    ok, raw = pcall(gm.GetNodeData, gm, zoneName)
  end
  if not ok or type(raw) ~= "table" then return {} end
  -- Normalize a few known shapes into ours; ignore anything unrecognized.
  local out = {}
  for key, val in pairs(raw) do
    if type(val) == "table" and type(val.x) == "number" and type(val.y) == "number" then
      table.insert(out, { x = val.x, y = val.y, kind = val.kind or val.nodeType or "unknown" })
    elseif type(key) == "number" and type(val) == "number" then
      -- Packed coordinate form used by some node DBs (x*10000+y packed): decode
      -- defensively; out-of-range values are dropped.
      local packed = key
      local x = math.floor(packed / 10000) / 100
      local y = (packed % 10000) / 100
      if x >= 0 and x <= 100 and y >= 0 and y <= 100 then
        table.insert(out, { x = x, y = y, kind = "unknown" })
      end
    end
  end
  return out
end

function Adapter.Status()
  local caps = Adapter.Capabilities()
  if not caps.available then return "GatherMate2: not installed (optional)" end
  return string.format("GatherMate2: connected (nodeQuery=%s, displayControl=%s)",
    tostring(caps.nodeQuery), tostring(caps.displayControl))
end
