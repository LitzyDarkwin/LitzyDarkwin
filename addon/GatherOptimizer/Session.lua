-- GatherOptimizer/Session.lua — session lifecycle + legitimate gather tracking.
-- Gathers are detected from CHAT_MSG_LOOT (self loot) with zone/map context.
-- No automation, no protected calls, no OnUpdate loops.
local _, GO = ...
if GO == nil then GO = _G.__GO_TEST_NS or {} end

local Session = {}
GO.Session = Session
GO.modules["Session"] = Session

Session._counter = 0

local function current()
  return Session._active
end

local function classifyResource(name, itemID)
  -- Deterministic keyword classifier (addon-side hint only; desktop re-maps).
  local n = (name or ""):lower()
  local herbs = { "herb", "flower", "blossom", "weed", "moss", "lichen", "lotus", "sage", "snowdrop", "stormdust", "arathor", "mycobloom" }
  local ores = { "ore", "bar", "stone", "crystal", "gem", "cobalt", "obsidium", "pyrite", "ghost iron", "trillium", "leystone", "felslate", "monelite", "storm silver", "platinum", "osmenite", "solenium", "oxxein", "draconium", "serevite", "null stone", "bismuth", "ironclaw" }
  for _, k in ipairs(ores) do
    if n:find(k, 1, true) then return "mining" end
  end
  for _, k in ipairs(herbs) do
    if n:find(k, 1, true) then return "herbalism" end
  end
  if itemID then
    -- Known gathering-result item families are resolved desktop-side; keep raw.
    return "unknown"
  end
  return "unknown"
end
Session.ClassifyResource = classifyResource

--- Parse "You receive loot: <item link>[xN]." Returns itemID, name, qty or nil.
function Session.ParseLootMessage(msg)
  if type(msg) ~= "string" then return nil end
  -- Self-loot prefix (enUS/deDE/frFR share structure; pattern is prefix-anchored).
  local body = msg:match("^You receive loot:%s*(.+)$")
  if not body then return nil end
  local itemID = body:match("|Hitem:(%d+)")
  local name = body:match("%[(.-)%]")
  local qty = tonumber(body:match("x(%d+)%s*%.?%s*$")) or 1
  if not itemID and not name then return nil end
  return tonumber(itemID), name, qty
end

local function newSessionID(player, realm, now)
  Session._counter = Session._counter + 1
  return string.format("%s-%s-%d-%d", tostring(player), tostring(realm), now, Session._counter)
end

function Session.Start(note)
  local db = GO.Store.DB()
  if current() then
    GO.Log("A session is already recording. /go stop first.")
    return nil
  end
  local player, realm = GO.PlayerInfo()
  local now = GO.Now()
  local mapID, x, y, zone = GO.PlayerMapPosition()
  local session = {
    id = newSessionID(player, realm, now),
    character = player,
    realm = realm,
    zone = zone,
    mapID = mapID,
    startX = x,
    startY = y,
    note = note or "",
    startedAt = now,
    endedAt = nil,
    activeSeconds = 0,
    _lastTick = now,
    paused = false,
    gathers = {},
    gatherCount = 0,
    state = "recording",
  }
  table.insert(db.sessions, session)
  Session._active = session
  GO.Store.Prune()
  GO.Log(string.format("Session started (%s). Loot while farming is recorded.", zone or "unknown zone"))
  return session
end

local function accrue(s, now)
  if s and not s.paused and s.state == "recording" then
    local dt = now - (s._lastTick or now)
    if dt > 0 and dt < 3600 then
      s.activeSeconds = s.activeSeconds + dt
    end
  end
  if s then s._lastTick = now end
end

function Session.Pause()
  local s = current()
  if not s then GO.Log("No active session.") return end
  accrue(s, GO.Now())
  s.paused = true
  s.state = "paused"
  GO.Log("Session paused.")
end

function Session.Resume()
  local s = current()
  if not s then GO.Log("No active session.") return end
  s._lastTick = GO.Now()
  s.paused = false
  s.state = "recording"
  GO.Log("Session resumed.")
end

function Session.Stop()
  local s = current()
  if not s then GO.Log("No active session.") return end
  local now = GO.Now()
  accrue(s, now)
  s.endedAt = now
  s.state = "ended"
  s._lastTick = nil
  Session._active = nil
  GO.Log(string.format("Session ended: %d gathers in %s. Log out or /reload to flush data for the desktop app.",
    s.gatherCount or 0, Session.FormatDuration(s.activeSeconds or 0)))
  return s
end

function Session.FormatDuration(secs)
  secs = math.floor(tonumber(secs) or 0)
  local h = math.floor(secs / 3600)
  local m = math.floor((secs % 3600) / 60)
  local s = secs % 60
  if h > 0 then return string.format("%dh %02dm", h, m) end
  if m > 0 then return string.format("%dm %02ds", m, s) end
  return string.format("%ds", s)
end

function Session.Status()
  local s = current()
  if not s then
    local db = GO.Store.DB()
    return string.format("Idle. %d past session(s), %d observation(s) stored.",
      #db.sessions, #db.observations)
  end
  accrue(s, GO.Now())
  return string.format("%s: %d gathers, %s active (%s).",
    s.paused and "Paused" or "Recording",
    s.gatherCount or 0,
    Session.FormatDuration(s.activeSeconds or 0),
    s.zone or "unknown zone")
end

--- Record one loot event into the active session + observation ring.
function Session.RecordLoot(itemID, name, qty)
  local mapID, x, y, zone = GO.PlayerMapPosition()
  local now = GO.Now()
  local s = current()
  if s and not s.paused then
    accrue(s, now)
    local db = GO.Store.DB()
    local maxG = tonumber(db.settings.maxGathersPerSession) or 2000
    if #(s.gathers or {}) < maxG then
      table.insert(s.gathers, {
        t = now, itemID = itemID, name = name, qty = qty,
        mapID = mapID, x = x, y = y, zone = zone,
        cat = classifyResource(name, itemID),
      })
    end
    s.gatherCount = (s.gatherCount or 0) + 1
  end
  -- Observations are recorded even outside sessions (cheap, capped ring).
  if x and y then
    GO.Store.AddObservation({
      t = now, mapID = mapID, x = x, y = y, zone = zone,
      itemID = itemID, name = name, qty = qty,
      cat = classifyResource(name, itemID),
    })
  end
end

local function onLoot(_, msg)
  local db = GO.Store and GO.Store.DB() or nil
  if db and not db.settings.recordGathers then return end
  local itemID, name, qty = Session.ParseLootMessage(msg)
  if itemID or name then
    Session.RecordLoot(itemID, name, qty or 1)
  end
end

function Session.Init()
  GO._sessionCmd = function(action, arg)
    if action == "start" then Session.Start(arg ~= "" and arg or nil)
    elseif action == "stop" then Session.Stop()
    elseif action == "pause" then Session.Pause()
    elseif action == "resume" then Session.Resume()
    else GO.Log(Session.Status()) end
  end
  -- Register exactly once: Init re-runs on every ADDON_LOADED.
  if not Session._lootHooked then
    Session._lootHooked = true
    GO.On("CHAT_MSG_LOOT", onLoot)
  end
  -- Resume nothing automatically: an "ended" session stays ended; a leftover
  -- "recording" session from a crashed client is closed on load.
  local db = GO.Store.DB()
  for _, s in ipairs(db.sessions) do
    if s.state == "recording" or s.state == "paused" then
      s.state = "ended"
      s.endedAt = s.endedAt or s.startedAt
      s._lastTick = nil
    end
  end
  Session._active = nil
end
