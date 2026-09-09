//! TSM public-data market provider + valuation (ALGORITHMS.md §5).
//!
//! Data: `commodities.csv` (live regional commodity prices, ~3 h) and
//! `region/items.csv` (daily region statistics). HTTP fetching lives in the
//! shell; this module parses, ingests, and valuates. Stale data degrades
//! gracefully; missing data is Unknown — never a fake price.

use crate::error::{AppError, Result};
use crate::models::{policies, Freshness};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Provider ----------------------------------------------------------------------

pub const PROVIDER_TSM: &str = "tsm";
pub const GAME_RETAIL: &str = "retail";
pub const DATASET_COMMODITIES: &str = "commodities";
pub const DATASET_REGION_ITEMS: &str = "region-items";

pub fn commodities_url(region: &str) -> String {
    format!("https://public-data.tradeskillmaster.com/{GAME_RETAIL}/{region}/commodities.csv")
}

pub fn region_items_url(region: &str) -> String {
    format!("https://public-data.tradeskillmaster.com/{GAME_RETAIL}/{region}/region/items.csv")
}

// Dependency-free CSV ---------------------------------------------------------------
// Handles quoted fields (commas, "" escapes, embedded newlines) and CRLF.

fn parse_csv_table(text: &str) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                field.push(c);
            }
        } else if c == '"' {
            in_quotes = true;
        } else if c == ',' {
            row.push(std::mem::take(&mut field));
        } else if c == '\n' {
            row.push(std::mem::take(&mut field));
            rows.push(std::mem::take(&mut row));
        } else if c != '\r' {
            field.push(c);
        }
    }
    // Trailing field/row (a bare trailing newline adds nothing).
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

/// Days-from-civil (Howard Hinnant) → unix seconds. Strict `…Z` UTC only.
fn parse_tsm_time(raw: &str) -> Option<i64> {
    // "2026-07-05T12:00:00Z" — exactly 20 chars.
    let b = raw.trim().as_bytes();
    if b.len() != 20 || b[19] != b'Z' || b[10] != b'T' {
        return None;
    }
    let num = |from: usize, to: usize| -> Option<i64> {
        std::str::from_utf8(&b[from..to]).ok()?.parse::<i64>().ok()
    };
    if b[4] != b'-' || b[7] != b'-' || b[13] != b':' || b[16] != b':' {
        return None;
    }
    let (y, m, d) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    let (hh, mm, ss) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    if hh > 23 || mm > 59 || ss > 60 {
        return None;
    }
    let y_adj = if m <= 2 { y - 1 } else { y };
    let era = y_adj.div_euclid(400);
    let yoe = y_adj.rem_euclid(400);
    let mp = (m + 9).rem_euclid(12);
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some(days * 86400 + hh * 3600 + mm * 60 + ss)
}

// Rows -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CommodityRow {
    pub item_id: i64,
    pub name: String,
    pub market_value: i64,
    pub min_buyout: i64,
    pub recent: i64,
    pub historical: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct RegionRow {
    pub item_id: i64,
    pub name: String,
    pub market_value: i64,
    pub historical: i64,
    pub avg_sale_price: i64,
    pub sale_rate: f64,
    pub sold_per_day: f64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Default)]
pub struct ParseReport {
    pub rows_total: usize,
    pub rows_ok: usize,
    pub rows_skipped: usize,
}

fn int_field(row: &[String], i: usize) -> Option<i64> {
    let s = row.get(i)?.trim();
    if s.is_empty() {
        return Some(0);
    }
    s.parse::<i64>().ok()
}

fn float_field(row: &[String], i: usize) -> Option<f64> {
    let s = row.get(i)?.trim();
    if s.is_empty() {
        return Some(0.0);
    }
    s.parse::<f64>().ok().filter(|v| v.is_finite())
}

fn check_header(rows: &[Vec<String>], want: &[&str], file: &str) -> Result<()> {
    match rows.first() {
        Some(h) if h.iter().map(String::as_str).collect::<Vec<_>>() == want => Ok(()),
        Some(h) => Err(AppError::market(format!(
            "{file}: unexpected header [{}] (provider format changed?)",
            h.join(",")
        ))),
        None => Err(AppError::market(format!("{file}: empty file"))),
    }
}

/// Parse `commodities.csv`. Bad rows are skipped + counted; a bad header fails.
pub fn parse_commodities(text: &str) -> Result<(Vec<CommodityRow>, ParseReport)> {
    let rows = parse_csv_table(text);
    check_header(
        &rows,
        &["itemId", "name", "marketValue", "minBuyout", "recent", "historical", "updatedAt"],
        "commodities.csv",
    )?;
    let mut out = Vec::new();
    let mut report = ParseReport { rows_total: rows.len().saturating_sub(1), ..Default::default() };
    for row in rows.iter().skip(1) {
        let parsed = (|| -> Option<CommodityRow> {
            if row.len() != 7 {
                return None;
            }
            let item_id = int_field(row, 0)?;
            if item_id <= 0 {
                return None;
            }
            Some(CommodityRow {
                item_id,
                name: row[1].trim().to_string(),
                market_value: int_field(row, 2)?,
                min_buyout: int_field(row, 3)?,
                recent: int_field(row, 4)?,
                historical: int_field(row, 5)?,
                updated_at: parse_tsm_time(&row[6])?,
            })
        })();
        match parsed {
            Some(r) => {
                out.push(r);
                report.rows_ok += 1;
            }
            None => report.rows_skipped += 1,
        }
    }
    Ok((out, report))
}

/// Parse `region/items.csv`. Bad rows are skipped + counted; a bad header fails.
pub fn parse_region_items(text: &str) -> Result<(Vec<RegionRow>, ParseReport)> {
    let rows = parse_csv_table(text);
    check_header(
        &rows,
        &[
            "itemId",
            "name",
            "marketValue",
            "historical",
            "avgSalePrice",
            "saleRate",
            "soldPerDay",
            "updatedAt",
        ],
        "region/items.csv",
    )?;
    let mut out = Vec::new();
    let mut report = ParseReport { rows_total: rows.len().saturating_sub(1), ..Default::default() };
    for row in rows.iter().skip(1) {
        let parsed = (|| -> Option<RegionRow> {
            if row.len() != 8 {
                return None;
            }
            let item_id = int_field(row, 0)?;
            if item_id <= 0 {
                return None;
            }
            Some(RegionRow {
                item_id,
                name: row[1].trim().to_string(),
                market_value: int_field(row, 2)?,
                historical: int_field(row, 3)?,
                avg_sale_price: int_field(row, 4)?,
                sale_rate: float_field(row, 5)?,
                sold_per_day: float_field(row, 6)?,
                updated_at: parse_tsm_time(&row[7])?,
            })
        })();
        match parsed {
            Some(r) => {
                out.push(r);
                report.rows_ok += 1;
            }
            None => report.rows_skipped += 1,
        }
    }
    Ok((out, report))
}

// Ingest ------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestReport {
    pub dataset: String,
    pub rows_total: usize,
    pub rows_upserted: usize,
    pub rows_skipped: usize,
    pub snapshot_updated_at: Option<i64>,
}

fn log_refresh(
    conn: &Connection,
    region: &str,
    dataset: &str,
    started_at: i64,
    finished_at: i64,
    status: &str,
    total: usize,
    upserted: usize,
    skipped: usize,
    error: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO market_refresh (provider, region, dataset, started_at, finished_at,
         status, rows_total, rows_upserted, rows_skipped, error)
         VALUES ('tsm', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            region,
            dataset,
            started_at,
            finished_at,
            status,
            total as i64,
            upserted as i64,
            skipped as i64,
            error,
        ],
    )?;
    Ok(())
}

fn record_provider(
    conn: &Connection,
    dataset: &str,
    now: i64,
    ok: bool,
    error: Option<&str>,
) -> Result<()> {
    let failures: i64 = conn
        .query_row(
            "SELECT failures FROM provider_state WHERE provider = 'tsm' AND dataset = ?1",
            rusqlite::params![dataset],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if ok {
        conn.execute(
            "INSERT INTO provider_state (provider, dataset, last_success_at, last_attempt_at,
             last_error, failures, next_retry_at)
             VALUES ('tsm', ?1, ?2, ?2, NULL, 0, 0)
             ON CONFLICT(provider, dataset) DO UPDATE SET
               last_success_at = excluded.last_success_at,
               last_attempt_at = excluded.last_attempt_at,
               last_error = NULL, failures = 0, next_retry_at = 0",
            rusqlite::params![dataset, now],
        )?;
    } else {
        let failures = failures + 1;
        let backoff = (300 * failures).min(3600);
        conn.execute(
            "INSERT INTO provider_state (provider, dataset, last_success_at, last_attempt_at,
             last_error, failures, next_retry_at)
             VALUES ('tsm', ?1, NULL, ?2, ?3, 1, ?4)
             ON CONFLICT(provider, dataset) DO UPDATE SET
               last_attempt_at = excluded.last_attempt_at,
               last_error = excluded.last_error,
               failures = ?5, next_retry_at = excluded.next_retry_at",
            rusqlite::params![dataset, now, error, now + backoff, failures],
        )?;
    }
    Ok(())
}

fn fail_ingest(
    conn: &Connection,
    region: &str,
    dataset: &str,
    started_at: i64,
    err: AppError,
) -> Result<IngestReport> {
    let msg = err.user_message();
    log_refresh(conn, region, dataset, started_at, started_at, "failed", 0, 0, 0, Some(&msg))?;
    record_provider(conn, dataset, started_at, false, Some(&msg))?;
    Err(err)
}

/// Ingest a fetched `commodities.csv` snapshot (transactional; old cache
/// survives any failure).
pub fn ingest_commodities(
    conn: &mut Connection,
    region: &str,
    text: &str,
    now: i64,
) -> Result<IngestReport> {
    let started_at = now;
    let (rows, parse) = match parse_commodities(text) {
        Ok(v) => v,
        Err(e) => return fail_ingest(conn, region, DATASET_COMMODITIES, started_at, e),
    };
    if rows.is_empty() {
        return fail_ingest(
            conn,
            region,
            DATASET_COMMODITIES,
            started_at,
            AppError::market("commodities.csv: no usable rows"),
        );
    }
    let snapshot = rows.iter().map(|r| r.updated_at).max();
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO market_items
               (item_id, region, market_value, min_buyout, recent, historical,
                item_name, market_updated_at, fetched_at, provider)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'tsm')
             ON CONFLICT(item_id) DO UPDATE SET
               region = excluded.region,
               market_value = excluded.market_value,
               min_buyout = excluded.min_buyout,
               recent = excluded.recent,
               historical = excluded.historical,
               item_name = excluded.item_name,
               market_updated_at = excluded.market_updated_at,
               fetched_at = excluded.fetched_at",
        )?;
        for r in &rows {
            stmt.execute(rusqlite::params![
                r.item_id,
                region,
                r.market_value,
                r.min_buyout,
                r.recent,
                r.historical,
                r.name,
                r.updated_at,
                now,
            ])?;
        }
    }
    log_refresh(
        &tx,
        region,
        DATASET_COMMODITIES,
        started_at,
        now,
        "success",
        parse.rows_total,
        parse.rows_ok,
        parse.rows_skipped,
        None,
    )?;
    record_provider(&tx, DATASET_COMMODITIES, now, true, None)?;
    tx.commit()?;
    Ok(IngestReport {
        dataset: DATASET_COMMODITIES.to_string(),
        rows_total: parse.rows_total,
        rows_upserted: parse.rows_ok,
        rows_skipped: parse.rows_skipped,
        snapshot_updated_at: snapshot,
    })
}

/// Ingest a fetched `region/items.csv` snapshot. Only the daily-statistics
/// columns are touched — region data never overwrites (or impersonates) the
/// live commodity columns. Rows the live file never covered valuate from
/// history with an explicit `live-unknown` flag.
pub fn ingest_region_items(
    conn: &mut Connection,
    region: &str,
    text: &str,
    now: i64,
) -> Result<IngestReport> {
    let started_at = now;
    let (rows, parse) = match parse_region_items(text) {
        Ok(v) => v,
        Err(e) => return fail_ingest(conn, region, DATASET_REGION_ITEMS, started_at, e),
    };
    if rows.is_empty() {
        return fail_ingest(
            conn,
            region,
            DATASET_REGION_ITEMS,
            started_at,
            AppError::market("region/items.csv: no usable rows"),
        );
    }
    let snapshot = rows.iter().map(|r| r.updated_at).max();
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO market_items
               (item_id, region, avg_sale_price, sale_rate, sold_per_day,
                item_name, region_updated_at, fetched_at, provider)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'tsm')
             ON CONFLICT(item_id) DO UPDATE SET
               region = excluded.region,
               avg_sale_price = excluded.avg_sale_price,
               sale_rate = excluded.sale_rate,
               sold_per_day = excluded.sold_per_day,
               item_name = excluded.item_name,
               region_updated_at = excluded.region_updated_at,
               fetched_at = excluded.fetched_at",
        )?;
        for r in &rows {
            stmt.execute(rusqlite::params![
                r.item_id,
                region,
                r.avg_sale_price,
                r.sale_rate,
                r.sold_per_day,
                r.name,
                r.updated_at,
                now,
            ])?;
        }
    }
    log_refresh(
        &tx,
        region,
        DATASET_REGION_ITEMS,
        started_at,
        now,
        "success",
        parse.rows_total,
        parse.rows_ok,
        parse.rows_skipped,
        None,
    )?;
    record_provider(&tx, DATASET_REGION_ITEMS, now, true, None)?;
    tx.commit()?;
    Ok(IngestReport {
        dataset: DATASET_REGION_ITEMS.to_string(),
        rows_total: parse.rows_total,
        rows_upserted: parse.rows_ok,
        rows_skipped: parse.rows_skipped,
        snapshot_updated_at: snapshot,
    })
}

// Valuation -------------------------------------------------------------------------------

/// All valuation weights live here — no mystery constants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValuationConfig {
    pub spike_clamp: f64,
    pub spike_floor: f64,
    pub refresh_hours: f64,
    pub stale_floor: f64,
    pub haircut_base: f64,
    pub haircut_liquidity: f64,
}

impl Default for ValuationConfig {
    fn default() -> Self {
        ValuationConfig {
            spike_clamp: 2.5,
            spike_floor: 1.0 / 3.0,
            refresh_hours: 12.0,
            stale_floor: 0.5,
            haircut_base: 0.90,
            haircut_liquidity: 0.10,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MarketSignals {
    pub market_value: i64,
    pub min_buyout: i64,
    pub recent: i64,
    pub historical: i64,
    pub avg_sale_price: i64,
    pub sale_rate: f64,
    pub sold_per_day: f64,
    pub market_updated_at: i64,
    pub region_updated_at: i64,
    pub fetched_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemValuation {
    pub item_id: i64,
    /// Clamped live signal (display price).
    pub market_value: i64,
    /// Conservative basket × liquidity haircut.
    pub conservative_value: i64,
    /// Conservative × freshness penalty — what routes are valued at.
    pub effective_value: i64,
    pub confidence: f64,
    pub freshness: Freshness,
    pub flags: Vec<String>,
    pub explain: Vec<String>,
}

/// ALGORITHMS.md §5. `None` (or all-zero signals) → Unknown, never a fake price.
pub fn valuate(
    item_id: i64,
    signals: Option<&MarketSignals>,
    cfg: &ValuationConfig,
    now: i64,
) -> ItemValuation {
    let unknown = |flags: Vec<String>, explain: Vec<String>| ItemValuation {
        item_id,
        market_value: 0,
        conservative_value: 0,
        effective_value: 0,
        confidence: 0.0,
        freshness: Freshness::Unknown,
        flags,
        explain,
    };
    let s = match signals {
        Some(s) => s,
        None => {
            return unknown(
                vec!["no-data".to_string()],
                vec!["No market data cached for this item yet.".to_string()],
            )
        }
    };
    let prices = [s.market_value, s.min_buyout, s.recent, s.historical, s.avg_sale_price];
    if prices.iter().all(|&p| p <= 0) {
        return unknown(
            vec!["no-data".to_string()],
            vec!["Cached row has no usable price signals.".to_string()],
        );
    }
    let mut flags: Vec<String> = Vec::new();
    let mut explain: Vec<String> = Vec::new();

    // 1. Live signal + spike clamp against the historical baseline.
    let live = s.market_value.max(0) as f64;
    let baseline = s.historical.max(0).max(s.avg_sale_price.max(0)) as f64;
    let mut clamped = live;
    if baseline > 0.0 && live > 0.0 {
        let hi = baseline * cfg.spike_clamp.max(1.0);
        let lo = baseline * cfg.spike_floor.clamp(0.0, 1.0);
        if live > hi {
            clamped = hi;
            flags.push("spiked".to_string());
            explain.push(format!(
                "Live {} spiked over ×{:.1} baseline; clamped to {}.",
                live as i64,
                cfg.spike_clamp,
                clamped as i64
            ));
        } else if live < lo {
            clamped = lo;
            flags.push("floored".to_string());
            explain.push(format!(
                "Live {} undercuts the baseline floor; raised to {}.",
                live as i64,
                clamped as i64
            ));
        } else {
            explain.push(format!("Live market value {} within baseline band.", live as i64));
        }
    } else {
        if baseline <= 0.0 {
            flags.push("no-baseline".to_string());
            explain.push("No historical baseline; spike protection off.".to_string());
        }
        if live <= 0.0 {
            flags.push("live-unknown".to_string());
            explain.push("No live quote; valuing from history.".to_string());
        }
    }

    // 2. Conservative basket: min of the (positive) slow signals.
    let mut basket = Vec::new();
    if clamped > 0.0 {
        basket.push(("market", clamped));
    }
    if s.min_buyout > 0 {
        basket.push(("min-buyout", s.min_buyout as f64));
    }
    if s.avg_sale_price > 0 {
        basket.push(("avg-sale", s.avg_sale_price as f64));
    }
    if s.historical > 0 {
        basket.push(("historical", s.historical as f64));
    }
    // Basket is non-empty: some price signal is positive (checked above), and
    // every positive signal except `recent` feeds the basket.
    let (picked, conservative_pre) = if basket.is_empty() {
        // Only `recent` is positive: fall back to it rather than failing.
        flags.push("recent-only".to_string());
        ("recent", s.recent.max(0) as f64)
    } else {
        basket.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        (basket[0].0, basket[0].1)
    };
    let haircut = cfg.haircut_base + cfg.haircut_liquidity * s.sale_rate.clamp(0.0, 1.0);
    let conservative = (conservative_pre * haircut).round().max(0.0) as i64;
    explain.push(format!(
        "Conservative = min basket ({}) × {:.0}% liquidity haircut = {}.",
        picked,
        haircut * 100.0,
        conservative
    ));

    // 3. Freshness penalty.
    let data_ts = s.market_updated_at.max(s.region_updated_at).max(s.fetched_at);
    let (freshness, penalty) = if data_ts <= 0 {
        flags.push("unknown-age".to_string());
        (Freshness::Unknown, cfg.stale_floor)
    } else {
        let age_hours = (now - data_ts).max(0) as f64 / 3600.0;
        let over = age_hours / cfg.refresh_hours.max(0.5);
        let penalty = if over <= 1.0 {
            1.0
        } else {
            (1.0 - 0.125 * (over - 1.0)).max(cfg.stale_floor)
        };
        if penalty < 1.0 {
            flags.push("stale".to_string());
        }
        (policies::MARKET.classify(Some(data_ts), now), penalty)
    };
    let effective = (conservative as f64 * penalty).round().max(0.0) as i64;
    explain.push(format!(
        "Effective = conservative × {:.0}% freshness = {}.",
        penalty * 100.0,
        effective
    ));

    // 4. Confidence from agreement, liquidity, recency, coverage.
    let positives: Vec<f64> = prices.iter().filter(|&&p| p > 0).map(|&p| p as f64).collect();
    let coverage = positives.len() as f64 / 5.0;
    let agreement = if positives.len() >= 2 {
        let mean = positives.iter().sum::<f64>() / positives.len() as f64;
        let var = positives.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>()
            / positives.len() as f64;
        (1.0 - var.sqrt() / mean).max(0.0)
    } else {
        0.5
    };
    let liquidity =
        0.5 * (s.sold_per_day / 20.0).clamp(0.0, 1.0) + 0.5 * s.sale_rate.clamp(0.0, 1.0);
    if s.sold_per_day < 1.0 {
        flags.push("thin".to_string());
    }
    let confidence =
        0.35 * agreement + 0.25 * liquidity + 0.25 * penalty + 0.15 * coverage;
    explain.push(format!(
        "Confidence {:.0}% (agreement {:.0}%, liquidity {:.0}%, recency {:.0}%, coverage {:.0}%).",
        confidence * 100.0,
        agreement * 100.0,
        liquidity * 100.0,
        penalty * 100.0,
        coverage * 100.0
    ));

    ItemValuation {
        item_id,
        market_value: clamped.round().max(0.0) as i64,
        conservative_value: conservative,
        effective_value: effective,
        confidence: confidence.clamp(0.0, 1.0),
        freshness,
        flags,
        explain,
    }
}

// Queries -----------------------------------------------------------------------------

fn row_to_signals(row: &rusqlite::Row) -> rusqlite::Result<MarketSignals> {
    Ok(MarketSignals {
        market_value: row.get(0)?,
        min_buyout: row.get(1)?,
        recent: row.get(2)?,
        historical: row.get(3)?,
        avg_sale_price: row.get(4)?,
        sale_rate: row.get(5)?,
        sold_per_day: row.get(6)?,
        market_updated_at: row.get(7)?,
        region_updated_at: row.get(8)?,
        fetched_at: row.get(9)?,
    })
}

const SIGNAL_COLS: &str = "market_value, min_buyout, recent, historical, avg_sale_price,
    sale_rate, sold_per_day, market_updated_at, region_updated_at, fetched_at";

pub fn get_signals(conn: &Connection, item_id: i64) -> Result<Option<MarketSignals>> {
    let sql = format!("SELECT {SIGNAL_COLS} FROM market_items WHERE item_id = ?1");
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(rusqlite::params![item_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row_to_signals(row)?)),
        None => Ok(None),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketItemRow {
    pub item_id: i64,
    pub item_name: String,
    pub market_value: i64,
    pub effective_value: i64,
    pub confidence: f64,
    pub freshness: Freshness,
}

/// Search cached items by name (substring) or exact id. Results carry live
/// valuations, ranked by effective value desc.
pub fn search_items(
    conn: &Connection,
    query: &str,
    limit: usize,
    cfg: &ValuationConfig,
    now: i64,
) -> Result<Vec<MarketItemRow>> {
    let limit = limit.clamp(1, 100);
    // Wildcards are neutralized (LIKE is still substring via the % wrap).
    let like = format!("%{}%", query.trim().replace(['%', '_'], " "));
    let id_match: Option<i64> = query.trim().parse().ok();
    let sql = format!(
        "SELECT item_id, item_name, {SIGNAL_COLS} FROM market_items
         WHERE item_name LIKE ?1 OR item_id = ?2
         ORDER BY market_value DESC LIMIT ?3"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params![like, id_match.unwrap_or(-1), limit as i64], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, row_to_signals_offset(r, 2)?))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (item_id, item_name, signals) = row?;
        let v = valuate(item_id, Some(&signals), cfg, now);
        out.push(MarketItemRow {
            item_id,
            item_name,
            market_value: v.market_value,
            effective_value: v.effective_value,
            confidence: v.confidence,
            freshness: v.freshness,
        });
    }
    out.sort_by(|a, b| b.effective_value.cmp(&a.effective_value));
    Ok(out)
}

fn row_to_signals_offset(row: &rusqlite::Row, o: usize) -> rusqlite::Result<MarketSignals> {
    Ok(MarketSignals {
        market_value: row.get(o)?,
        min_buyout: row.get(o + 1)?,
        recent: row.get(o + 2)?,
        historical: row.get(o + 3)?,
        avg_sale_price: row.get(o + 4)?,
        sale_rate: row.get(o + 5)?,
        sold_per_day: row.get(o + 6)?,
        market_updated_at: row.get(o + 7)?,
        region_updated_at: row.get(o + 8)?,
        fetched_at: row.get(o + 9)?,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetStatus {
    pub dataset: String,
    pub updated_at: Option<i64>,
    pub fetched_at: Option<i64>,
    pub freshness: Freshness,
    pub failures: i64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketStatus {
    pub provider: String,
    pub region: String,
    pub item_count: i64,
    pub datasets: Vec<DatasetStatus>,
    pub freshness: Freshness,
}

fn dataset_status(
    conn: &Connection,
    region: &str,
    dataset: &str,
    updated_col: &str,
    now: i64,
) -> Result<DatasetStatus> {
    let sql = format!(
        "SELECT MAX({updated_col}), MAX(fetched_at) FROM market_items WHERE region = ?1"
    );
    let (updated_at, fetched_at): (Option<i64>, Option<i64>) = conn
        .query_row(&sql, rusqlite::params![region], |r| Ok((r.get(0)?, r.get(1)?)))?;
    let updated_at = updated_at.filter(|&t| t > 0);
    let fetched_at = fetched_at.filter(|&t| t > 0);
    let (failures, last_error): (i64, Option<String>) = conn
        .query_row(
            "SELECT failures, last_error FROM provider_state
             WHERE provider = 'tsm' AND dataset = ?1",
            rusqlite::params![dataset],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap_or((0, None));
    let freshness = policies::MARKET.classify(updated_at.or(fetched_at), now);
    Ok(DatasetStatus { dataset: dataset.to_string(), updated_at, fetched_at, freshness, failures, last_error })
}

pub fn market_status(conn: &Connection, region: &str, now: i64) -> Result<MarketStatus> {
    let item_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM market_items WHERE region = ?1",
        rusqlite::params![region],
        |r| r.get(0),
    )?;
    let live = dataset_status(conn, region, DATASET_COMMODITIES, "market_updated_at", now)?;
    let daily = dataset_status(conn, region, DATASET_REGION_ITEMS, "region_updated_at", now)?;
    // Overall freshness follows the live dataset; the daily file alone still
    // beats nothing.
    let freshness = if live.updated_at.or(live.fetched_at).is_some() {
        live.freshness
    } else {
        daily.freshness
    };
    Ok(MarketStatus {
        provider: PROVIDER_TSM.to_string(),
        region: region.to_string(),
        item_count,
        datasets: vec![live, daily],
        freshness,
    })
}

// Resource → price bridge ------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketPrices {
    pub prices: HashMap<String, i64>,
    /// `resource_id: reason` entries for anything unpriceable (never silent).
    pub missing: Vec<String>,
}

/// Resolve route resources to effective copper/unit via the item mapping.
/// Partial success by design: priced resources flow through, the rest are
/// listed with reasons.
pub fn resolve_prices(
    conn: &Connection,
    resource_ids: &[String],
    cfg: &ValuationConfig,
    now: i64,
) -> Result<MarketPrices> {
    let mut out = MarketPrices { prices: HashMap::new(), missing: Vec::new() };
    for resource_id in resource_ids {
        let item_id: Option<Option<i64>> = conn
            .query_row(
                "SELECT item_id FROM resources WHERE id = ?1",
                rusqlite::params![resource_id],
                |r| r.get(0),
            )
            .optional()?;
        match item_id {
            None => out.missing.push(format!("{resource_id}: unknown resource")),
            Some(None) => out.missing.push(format!("{resource_id}: no item mapping")),
            Some(Some(item)) => {
                let signals = get_signals(conn, item)?;
                let v = valuate(item, signals.as_ref(), cfg, now);
                if v.effective_value > 0 {
                    out.prices.insert(resource_id.clone(), v.effective_value);
                } else if signals.is_none() {
                    out.missing.push(format!("{resource_id}: no market data (refresh market)"));
                } else {
                    out.missing.push(format!("{resource_id}: valued at zero"));
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::open_memory;

    /// 2026-09-08T12:00:00Z (commodity snapshot) and +6h/+1h/+48h offsets.
    const SNAP: i64 = 1788868800;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    fn read_fixture(name: &str) -> String {
        let path = format!("{}/../../fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read_to_string(&path).unwrap()
    }

    fn mem_db() -> Connection {
        let mut conn = open_memory().unwrap();
        crate::migrations::migrate(&mut conn).unwrap();
        conn
    }

    fn fresh_signals() -> MarketSignals {
        MarketSignals {
            market_value: 100_000,
            min_buyout: 95_000,
            recent: 98_000,
            historical: 80_000,
            avg_sale_price: 82_000,
            sale_rate: 0.5,
            sold_per_day: 10.0,
            market_updated_at: SNAP,
            region_updated_at: SNAP - 43200,
            fetched_at: SNAP,
        }
    }

    #[test]
    fn csv_handles_quotes_commas_and_crlf() {
        let text = "a,b,c\r\n1,\"two, too\",\"say \"\"hi\"\"\"\r\n2,plain,3\r\n";
        let table = parse_csv_table(text);
        assert_eq!(table.len(), 3);
        assert_eq!(table[0], vec!["a", "b", "c"]);
        assert_eq!(table[1], vec!["1", "two, too", "say \"hi\""]);
        assert_eq!(table[2], vec!["2", "plain", "3"]);
    }

    #[test]
    fn tsm_time_parses_strict_utc() {
        assert_eq!(parse_tsm_time("2026-07-05T12:00:00Z"), Some(1783252800));
        assert_eq!(parse_tsm_time("2026-09-08T12:00:00Z"), Some(SNAP));
        assert_eq!(parse_tsm_time("2026-09-08 12:00:00"), None);
        assert_eq!(parse_tsm_time("2026-13-08T12:00:00Z"), None);
        assert_eq!(parse_tsm_time(""), None);
        assert_eq!(parse_tsm_time("2026-09-08T12:00:00+00:00"), None);
    }

    #[test]
    fn provider_urls_follow_tsm_layout() {
        assert_eq!(
            commodities_url("eu"),
            "https://public-data.tradeskillmaster.com/retail/eu/commodities.csv"
        );
        assert_eq!(
            region_items_url("eu"),
            "https://public-data.tradeskillmaster.com/retail/eu/region/items.csv"
        );
    }

    #[test]
    fn parses_commodity_sample() {
        let text = read_fixture("tsm_commodities_sample.csv");
        let (rows, report) = parse_commodities(&text).unwrap();
        assert_eq!((report.rows_total, report.rows_ok, report.rows_skipped), (7, 5, 2));
        assert_eq!(rows[1].name, "Ironclaw, Ore");
        assert_eq!(rows[1].market_value, 48_000);
        assert_eq!(rows[3].market_value, 0); // Luredrop: live unknown
        assert_eq!(rows[4].min_buyout, 0); // empty field → 0
        assert!(rows.iter().all(|r| r.updated_at == SNAP));
    }

    #[test]
    fn parses_region_sample() {
        let text = read_fixture("tsm_region_sample.csv");
        let (rows, report) = parse_region_items(&text).unwrap();
        assert_eq!((report.rows_total, report.rows_ok, report.rows_skipped), (4, 4, 0));
        assert!(approx(rows[0].sold_per_day, 18.5, 1e-12));
        assert!(approx(rows[2].sale_rate, 0.61, 1e-12));
    }

    #[test]
    fn header_drift_fails_loudly() {
        let bad = "itemId,name,marketValue\n1,x,2\n";
        let err = parse_commodities(bad).unwrap_err();
        assert!(err.user_message().contains("unexpected header"), "{}", err.user_message());
        assert!(parse_commodities("").is_err());
    }

    fn ingested_db(now: i64) -> Connection {
        let mut conn = mem_db();
        let comm = read_fixture("tsm_commodities_sample.csv");
        let region = read_fixture("tsm_region_sample.csv");
        let rc = ingest_commodities(&mut conn, "eu", &comm, now).unwrap();
        assert_eq!((rc.rows_upserted, rc.rows_skipped), (5, 2));
        assert_eq!(rc.snapshot_updated_at, Some(SNAP));
        let rr = ingest_region_items(&mut conn, "eu", &region, now).unwrap();
        assert_eq!((rr.rows_upserted, rr.rows_skipped), (4, 0));
        conn
    }

    #[test]
    fn ingest_merges_live_and_daily() {
        let conn = ingested_db(SNAP + 3600);
        let s = get_signals(&conn, 161502).unwrap().expect("bismuth cached");
        assert_eq!(s.market_value, 52_000); // live wins over region 50500
        assert_eq!(s.avg_sale_price, 49_800);
        assert!(approx(s.sold_per_day, 18.5, 1e-12));
        assert_eq!(s.market_updated_at, SNAP);
        assert_eq!(s.region_updated_at, SNAP - 43200);
        // Luredrop: live zeros preserved (region file must not impersonate live).
        let l = get_signals(&conn, 161505).unwrap().expect("luredrop cached");
        assert_eq!(l.market_value, 0);
        assert_eq!(l.historical, 95_000);
        assert_eq!(l.avg_sale_price, 92_000);
        // Hochenblume only exists in the live file → no daily stats.
        let h = get_signals(&conn, 161507).unwrap().expect("hochenblume cached");
        assert_eq!(h.avg_sale_price, 0);
        assert_eq!(h.region_updated_at, 0);
        assert!(get_signals(&conn, 999_999).unwrap().is_none());
    }

    #[test]
    fn ingest_failure_preserves_cache_and_logs() {
        let mut conn = ingested_db(SNAP + 3600);
        let before = get_signals(&conn, 161502).unwrap().unwrap().market_value;
        let err = ingest_commodities(&mut conn, "eu", "itemId,name\n1,x\n", SNAP + 7200)
            .unwrap_err();
        assert!(err.user_message().contains("unexpected header"));
        let after = get_signals(&conn, 161502).unwrap().unwrap().market_value;
        assert_eq!(before, after);
        let (status, failures): (String, i64) = conn
            .query_row(
                "SELECT status, (SELECT failures FROM provider_state
                 WHERE provider = 'tsm' AND dataset = 'commodities')
                 FROM market_refresh ORDER BY id DESC LIMIT 1",
                rusqlite::params![],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, "failed");
        assert_eq!(failures, 1);
    }

    #[test]
    fn valuation_hand_case() {
        // baseline = 82000; band [27333, 205000]; live 100000 inside.
        // basket min = historical 80000; haircut 0.95 → 76000; fresh → same.
        let v = valuate(1, Some(&fresh_signals()), &ValuationConfig::default(), SNAP + 6 * 3600);
        assert_eq!(v.market_value, 100_000);
        assert_eq!(v.conservative_value, 76_000);
        assert_eq!(v.effective_value, 76_000);
        assert_eq!(v.freshness, Freshness::Fresh);
        assert!(v.flags.is_empty());
        // mean 91000, std 8342.66 → agreement 0.90832 → confidence 0.842913.
        assert!(approx(v.confidence, 0.842913, 0.001), "{}", v.confidence);
        assert!(!v.explain.is_empty());
    }

    #[test]
    fn valuation_spike_stale_thin() {
        let s = MarketSignals {
            market_value: 300_000,
            min_buyout: 0,
            recent: 0,
            historical: 80_000,
            avg_sale_price: 82_000,
            sale_rate: 0.2,
            sold_per_day: 0.5,
            market_updated_at: SNAP,
            region_updated_at: 0,
            fetched_at: SNAP,
        };
        // clamped to 2.5 × 82000 = 205000; basket 80000 × 0.92 = 73600;
        // 48h old → penalty 0.625 → 46000.
        let v = valuate(2, Some(&s), &ValuationConfig::default(), SNAP + 48 * 3600);
        assert_eq!(v.market_value, 205_000);
        assert_eq!(v.conservative_value, 73_600);
        assert_eq!(v.effective_value, 46_000);
        assert_eq!(v.freshness, Freshness::Aging);
        assert_eq!(v.flags, vec!["spiked", "stale", "thin"]);
    }

    #[test]
    fn valuation_unknown_without_signals() {
        let cfg = ValuationConfig::default();
        let v = valuate(3, None, &cfg, SNAP);
        assert_eq!(v.effective_value, 0);
        assert_eq!(v.confidence, 0.0);
        assert_eq!(v.freshness, Freshness::Unknown);
        assert_eq!(v.flags, vec!["no-data"]);
        let zero = MarketSignals::default();
        let v2 = valuate(3, Some(&zero), &cfg, SNAP);
        assert_eq!(v2.flags, vec!["no-data"]);
    }

    #[test]
    fn valuations_match_shared_fixture() {
        let text = read_fixture("expected/valuation.json");
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        let cfg = ValuationConfig::default();
        for case in v["cases"].as_array().unwrap() {
            let id = case["id"].as_str().unwrap();
            let signals = if case["signals"].is_null() {
                None
            } else {
                let s = &case["signals"];
                Some(MarketSignals {
                    market_value: s["marketValue"].as_i64().unwrap(),
                    min_buyout: s["minBuyout"].as_i64().unwrap(),
                    recent: s["recent"].as_i64().unwrap(),
                    historical: s["historical"].as_i64().unwrap(),
                    avg_sale_price: s["avgSalePrice"].as_i64().unwrap(),
                    sale_rate: s["saleRate"].as_f64().unwrap(),
                    sold_per_day: s["soldPerDay"].as_f64().unwrap(),
                    market_updated_at: s["marketUpdatedAt"].as_i64().unwrap(),
                    region_updated_at: s["regionUpdatedAt"].as_i64().unwrap(),
                    fetched_at: s["fetchedAt"].as_i64().unwrap(),
                })
            };
            let got = valuate(0, signals.as_ref(), &cfg, case["now"].as_i64().unwrap());
            let ex = &case["expect"];
            assert_eq!(got.market_value, ex["market"].as_i64().unwrap(), "{id}: market");
            assert_eq!(got.conservative_value, ex["conservative"].as_i64().unwrap(), "{id}: cons");
            assert_eq!(got.effective_value, ex["effective"].as_i64().unwrap(), "{id}: eff");
            assert!(
                approx(got.confidence, ex["confidence"].as_f64().unwrap(), 1e-9),
                "{id}: conf {}",
                got.confidence
            );
            let flags: Vec<&str> =
                ex["flags"].as_array().unwrap().iter().map(|f| f.as_str().unwrap()).collect();
            assert_eq!(got.flags, flags, "{id}: flags");
            let freshness: Freshness = serde_json::from_value(ex["freshness"].clone()).unwrap();
            assert_eq!(got.freshness, freshness, "{id}: freshness");
        }
    }

    #[test]
    fn search_and_status() {
        let now = SNAP + 3600;
        let conn = ingested_db(now);
        let cfg = ValuationConfig::default();
        let hits = search_items(&conn, "bloom", 10, &cfg, now).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].item_name, "Mycobloom");
        assert!(hits[0].effective_value > 0);
        let by_id = search_items(&conn, "161502", 10, &cfg, now).unwrap();
        assert_eq!(by_id.len(), 1);
        assert!(search_items(&conn, "zzz-no-such-item", 10, &cfg, now).unwrap().is_empty());

        let status = market_status(&conn, "eu", now).unwrap();
        assert_eq!(status.item_count, 5);
        assert_eq!(status.freshness, Freshness::Fresh);
        assert_eq!(status.datasets.len(), 2);
        assert_eq!(status.datasets[0].dataset, "commodities");
        assert_eq!(status.datasets[0].updated_at, Some(SNAP));

        let empty = market_status(&conn, "us", now).unwrap();
        assert_eq!(empty.item_count, 0);
        assert_eq!(empty.freshness, Freshness::Unknown);
    }

    #[test]
    fn resolve_prices_reports_missing() {
        let now = SNAP + 3600;
        let conn = ingested_db(now);
        conn.execute(
            "INSERT INTO farm_categories (id, key, name) VALUES ('mining', 'mining', 'Mining')",
            rusqlite::params![],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO resources (id, category_id, key, name, item_id) VALUES
             ('bismuth', 'mining', 'bismuth', 'Bismuth', 161502),
             ('unmapped-ore', 'mining', 'unmapped-ore', 'Unmapped Ore', NULL),
             ('ghost-ore', 'mining', 'ghost-ore', 'Ghost Ore', 999999)",
            rusqlite::params![],
        )
        .unwrap();
        let cfg = ValuationConfig::default();
        let res = resolve_prices(
            &conn,
            &["bismuth".to_string(), "unmapped-ore".to_string(), "ghost-ore".to_string(), "nope".to_string()],
            &cfg,
            now,
        )
        .unwrap();
        let direct = valuate(161502, get_signals(&conn, 161502).unwrap().as_ref(), &cfg, now);
        assert_eq!(res.prices.get("bismuth"), Some(&direct.effective_value));
        assert!(direct.effective_value > 0);
        assert_eq!(res.missing.len(), 3);
        assert!(res.missing.iter().any(|m| m.contains("no item mapping")));
        assert!(res.missing.iter().any(|m| m.contains("no market data")));
        assert!(res.missing.iter().any(|m| m.contains("unknown resource")));
    }
}
