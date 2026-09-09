-- GatherOptimizer/Store.lua — SavedVariables schema, defaults, migrations, pruning.
local _, GO = ...
if GO == nil then GO = _G.__GO_TEST_NS or {} end

local Store = {}
GO.Store = Store
GO.modules["Store"] = Store

local SCHEMA_VERSION = 1

local DEFAULT_SETTINGS = {
  recordGathers = true,
  recordObservations = true,
  maxObservations = 5000,
  maxSessions = 50,
  maxGathersPerSession = 2000,
}

local function defaults()
  return {
    schemaVersion = SCHEMA_VERSION,
    settings = {
      recordGathers = DEFAULT_SETTINGS.recordGathers,
      recordObservations = DEFAULT_SETTINGS.recordObservations,
      maxObservations = DEFAULT_SETTINGS.maxObservations,
      maxSessions = DEFAULT_SETTINGS.maxSessions,
      maxGathersPerSession = DEFAULT_SETTINGS.maxGathersPerSession,
    },
    sessions = {},
    observations = {},
    activeRoute = nil,
    export = { version = 1, exportedAt = 0 },
  }
end

local function isTable(t) return type(t) == "table" end

--- Migrate an old DB table in place. Returns the migrated table.
function Store.Migrate(db)
  if not isTable(db) then return defaults() end
  if db.schemaVersion ~= SCHEMA_VERSION then
    -- v0/unknown -> v1: keep sessions/observations if they look sane.
    local fresh = defaults()
    if isTable(db.sessions) then fresh.sessions = db.sessions end
    if isTable(db.observations) then fresh.observations = db.observations end
    if isTable(db.settings) then
      for k, v in pairs(fresh.settings) do
        if db.settings[k] ~= nil then fresh.settings[k] = db.settings[k] end
      end
    end
    if isTable(db.activeRoute) then fresh.activeRoute = db.activeRoute end
    fresh.schemaVersion = SCHEMA_VERSION
    return fresh
  end
  -- v1: fill any missing keys (forward-tolerant).
  local fresh = defaults()
  for k, v in pairs(fresh) do
    if db[k] == nil then db[k] = v end
  end
  for k, v in pairs(fresh.settings) do
    if not isTable(db.settings) then db.settings = {} end
    if db.settings[k] == nil then db.settings[k] = v end
  end
  if not isTable(db.sessions) then db.sessions = {} end
  if not isTable(db.observations) then db.observations = {} end
  return db
end

function Store.Init()
  _G.GatherOptimizerDB = Store.Migrate(_G.GatherOptimizerDB)
  Store.Prune()
end

function Store.DB()
  if not isTable(_G.GatherOptimizerDB) then
    _G.GatherOptimizerDB = defaults()
  end
  return _G.GatherOptimizerDB
end

--- Enforce ring-buffer caps so SV files stay small (called on init + writes).
function Store.Prune()
  local db = Store.DB()
  local maxObs = tonumber(db.settings.maxObservations) or DEFAULT_SETTINGS.maxObservations
  while #db.observations > maxObs do
    table.remove(db.observations, 1)
  end
  local maxSes = tonumber(db.settings.maxSessions) or DEFAULT_SETTINGS.maxSessions
  while #db.sessions > maxSes do
    table.remove(db.sessions, 1)
  end
  for _, s in ipairs(db.sessions) do
    if isTable(s.gathers) then
      local maxG = tonumber(db.settings.maxGathersPerSession) or DEFAULT_SETTINGS.maxGathersPerSession
      while #s.gathers > maxG do
        table.remove(s.gathers, 1)
      end
    end
  end
end

function Store.AddObservation(obs)
  local db = Store.DB()
  if not db.settings.recordObservations then return false end
  table.insert(db.observations, obs)
  local maxObs = tonumber(db.settings.maxObservations) or DEFAULT_SETTINGS.maxObservations
  if #db.observations > maxObs then
    table.remove(db.observations, 1)
  end
  return true
end

--- Wipe volatile data but keep settings (used by tests + /go reset if ever added).
function Store.Reset()
  local settings = Store.DB().settings
  _G.GatherOptimizerDB = defaults()
  _G.GatherOptimizerDB.settings = settings
end
