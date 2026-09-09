-- GatherOptimizer/Routes.lua — OPTIONAL Routes addon adapter.
-- Lets users draw our generated route inside the Routes addon when present.
-- Fully defensive: every global/function is probed before use.
local _, GO = ...
if GO == nil then GO = _G.__GO_TEST_NS or {} end

local Adapter = {}
GO.Routes = Adapter
GO.modules["Routes"] = Adapter

local function routesGlobal()
  return _G.Routes
end

function Adapter.IsAvailable()
  return routesGlobal() ~= nil
end

function Adapter.Capabilities()
  local r = routesGlobal()
  local caps = { available = r ~= nil, createRoute = false, showRoute = false }
  if type(r) ~= "table" then return caps end
  for _, key in ipairs({ "AddRoute", "CreateRoute", "NewRoute", "ImportRoute" }) do
    if type(r[key]) == "function" then caps.createRoute = true break end
  end
  for _, key in ipairs({ "ShowRoute", "DrawRoute", "DisplayRoute", "ForegroundRoute" }) do
    if type(r[key]) == "function" then caps.showRoute = true break end
  end
  return caps
end

--- Push a waypoint list into Routes (best effort). Returns ok, message.
-- waypoints: { {x=0-100, y=0-100}, ... }, zoneName: string, routeName: string
function Adapter.PushRoute(routeName, zoneName, waypoints)
  local r = routesGlobal()
  if type(r) ~= "table" then return false, "Routes addon not installed (optional)" end
  if type(waypoints) ~= "table" or #waypoints < 2 then
    return false, "Need at least 2 waypoints"
  end
  -- Convert to 0-1 coords for the Routes API shape.
  local pts = {}
  for i, w in ipairs(waypoints) do
    if type(w.x) == "number" and type(w.y) == "number" then
      pts[i] = { x = w.x / 100, y = w.y / 100 }
    end
  end
  for _, key in ipairs({ "AddRoute", "CreateRoute", "NewRoute", "ImportRoute" }) do
    if type(r[key]) == "function" then
      local ok, err = pcall(r[key], r, routeName, zoneName, pts)
      if ok then return true, "Route pushed to Routes via " .. key end
      return false, "Routes." .. key .. " failed: " .. tostring(err)
    end
  end
  return false, "Installed Routes version has no supported route-creation API"
end

function Adapter.Status()
  local caps = Adapter.Capabilities()
  if not caps.available then return "Routes: not installed (optional)" end
  return string.format("Routes: connected (createRoute=%s, showRoute=%s)",
    tostring(caps.createRoute), tostring(caps.showRoute))
end
