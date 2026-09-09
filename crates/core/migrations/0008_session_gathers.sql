-- 0008_session_gathers.sql — schema v8: per-item gather counts for yield learning.
-- Bridge aggregation previously kept only summed qty; learning avg_yield needs
-- the event count too. Old rows keep gathers = 0 (unknown) and are excluded
-- from yield stats until a re-sync refreshes them.
ALTER TABLE farm_session_items ADD COLUMN gathers INTEGER NOT NULL DEFAULT 0;
