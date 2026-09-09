-- 0007_market_tsm.sql — schema v7: align the market cache with the real TSM
-- public-data columns (see docs/PROVIDERS.md). v3 predated the column docs:
--   commodities.csv: itemId,name,marketValue,minBuyout,recent,historical,updatedAt
--   region/items.csv: itemId,name,marketValue,historical,avgSalePrice,saleRate,soldPerDay,updatedAt
-- Also fixes provider_state's primary key: one row per (provider, dataset)
-- instead of a single row per provider.

ALTER TABLE market_items RENAME COLUMN sale_avg TO avg_sale_price;
ALTER TABLE market_items RENAME COLUMN quantity TO sold_per_day;
ALTER TABLE market_items ADD COLUMN recent INTEGER NOT NULL DEFAULT 0;
ALTER TABLE market_items ADD COLUMN item_name TEXT NOT NULL DEFAULT '';

CREATE TABLE provider_state_new (
  provider TEXT NOT NULL,
  dataset TEXT NOT NULL DEFAULT '',
  last_success_at INTEGER NULL,
  last_attempt_at INTEGER NOT NULL DEFAULT 0,
  last_error TEXT NULL,
  failures INTEGER NOT NULL DEFAULT 0,
  next_retry_at INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (provider, dataset)
);

INSERT INTO provider_state_new
  (provider, dataset, last_success_at, last_attempt_at, last_error, failures, next_retry_at)
  SELECT provider, dataset, last_success_at, last_attempt_at, last_error, failures, next_retry_at
  FROM provider_state;

DROP TABLE provider_state;
ALTER TABLE provider_state_new RENAME TO provider_state;
