-- GatherOptimizer/Navigation.lua — waypoint navigation via OPTIONAL TomTom.
-- Without TomTom we still track ordered waypoints and print text guidance.
local _, GO = ...
if GO == nil then GO = _G.__GO_TEST_NS or {} end

local Nav = {}
GO.Navigation = Nav
GO.modules["Navigation"] = Nav

Nav._waypoints = {}   -- { uid?, mapID, x, y, label }
Nav._index = 0        -- current target (1-based into _waypoints)
Nav._active = false

local MAX_WAYPOINTS = 40

function Nav.IsTomTomAvailable()
  local t = _G.TomTom
  return type(t) == "table" and type(t.AddWaypoint) == "function"
end

function Nav.Status()
  if Nav.IsTomTomAvailable() then
    return "TomTom connected."
  end
  return "TomTom not installed (optional) — text guidance only."
end

local function clearTomTom()
  local t = _G.TomTom
  if type(t) ~= "table" then return end
  for _, w in ipairs(Nav._waypoints) do
    if w.uid and type(t.RemoveWaypoint) == "function" then
      pcall(t.RemoveWaypoint, t, w.uid)
    end
  end
end

--- Load a waypoint list. waypoints: { {x=0-100, y=0-100, label?}, ... }
function Nav.SetWaypoints(mapID, waypoints, title)
  Nav.Stop(true)
  Nav._waypoints = {}
  for i, w in ipairs(waypoints or {}) do
    if i > MAX_WAYPOINTS then break end
    if type(w.x) == "number" and type(w.y) == "number" then
      table.insert(Nav._waypoints, {
        mapID = mapID,
        x = w.x,
        y = w.y,
        label = w.label or string.format("%s %d/%d", title or "WP", i, #waypoints),
      })
    end
  end
  return #Nav._waypoints
end

function Nav.Start()
  if #Nav._waypoints == 0 then
    GO.Log("No route loaded. Use /go import <code> from the desktop app first.")
    return false
  end
  Nav._active = true
  Nav._index = 1
  if Nav.IsTomTomAvailable() then
    local t = _G.TomTom
    for i, w in ipairs(Nav._waypoints) do
      local ok, uid = pcall(t.AddWaypoint, t, w.mapID, w.x / 100, w.y / 100, {
        title = w.label,
        persistent = false,
        minimap = true,
        world = true,
        crazy = (i == 1),
      })
      if ok then w.uid = uid end
    end
    Nav.PointArrow()
  end
  local w = Nav._waypoints[1]
  GO.Log(string.format("Navigation started: %d waypoints. Next: %s (%.1f, %.1f).",
    #Nav._waypoints, w.label, w.x, w.y))
  return true
end

function Nav.PointArrow()
  if not Nav._active or not Nav.IsTomTomAvailable() then return end
  local t = _G.TomTom
  local w = Nav._waypoints[Nav._index]
  if w and w.uid and type(t.SetCrazyArrow) == "function" then
    pcall(t.SetCrazyArrow, t, w.uid, 100, w.label)
  end
end

--- Advance to the next waypoint (manual: gathering is never automated).
function Nav.Next()
  if not Nav._active then GO.Log("Navigation is not active.") return end
  local prev = Nav._waypoints[Nav._index]
  if prev and prev.uid and _G.TomTom and type(_G.TomTom.RemoveWaypoint) == "function" then
    pcall(_G.TomTom.RemoveWaypoint, _G.TomTom, prev.uid)
    prev.uid = nil
  end
  Nav._index = Nav._index + 1
  if Nav._index > #Nav._waypoints then
    GO.Log("Route complete. Nice farming.")
    Nav.Stop(true)
    return
  end
  Nav.PointArrow()
  local w = Nav._waypoints[Nav._index]
  GO.Log(string.format("Next: %s (%.1f, %.1f) — %d of %d.",
    w.label, w.x, w.y, Nav._index, #Nav._waypoints))
end

function Nav.Stop(silent)
  clearTomTom()
  Nav._active = false
  Nav._index = 0
  if _G.TomTom and type(_G.TomTom.ClearCrazyArrow) == "function" then
    pcall(_G.TomTom.ClearCrazyArrow, _G.TomTom)
  end
  if not silent then GO.Log("Navigation stopped.") end
end

function Nav.State()
  return {
    active = Nav._active,
    index = Nav._index,
    total = #Nav._waypoints,
    tomtom = Nav.IsTomTomAvailable(),
  }
end

function Nav.Slash(rest)
  local sub = ((rest or ""):match("^(%S+)") or "status"):lower()
  if sub == "start" then Nav.Start()
  elseif sub == "stop" then Nav.Stop(false)
  elseif sub == "next" then Nav.Next()
  else
    local st = Nav.State()
    GO.Log(string.format("Nav: active=%s waypoint %d/%d, TomTom=%s.",
      tostring(st.active), st.index, st.total, tostring(st.tomtom)))
  end
end
