#!/usr/bin/env python3
"""Generate crates/core/src/seed/atlas.json deterministically (seed=20260909).

Hand-authored zone/resource catalog + representative node priors. Priors are
clustered pseudo-observations around hand-picked centers using a fixed-seed PRNG,
so the output is byte-identical on every run. Commit the output; the desktop
treats every point as source='seed' with explicit attribution.

Zone list is intentionally small (6 zones, War Within + Dragonflight). It is a
starting prior, not a database: real coverage comes from user sync (addon),
crowd priors later, and future providers. Zone identities were chosen for
stability (well-established Retail zones); map IDs are included best-effort and
flagged for re-verification in PROVIDERS.md.
"""
import json
import random
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "crates" / "core" / "src" / "seed" / "atlas.json"
RNG = random.Random(20260909)

EXPANSIONS = [
    {"id": "tww", "name": "The War Within", "short": "TWW", "sort_order": 100},
    {"id": "df", "name": "Dragonflight", "short": "DF", "sort_order": 90},
]

CONTINENTS = [
    {"id": "khaz-algar", "expansion_id": "tww", "name": "Khaz Algar"},
    {"id": "dragon-isles", "expansion_id": "df", "name": "Dragon Isles"},
]

# (zone_id, continent, name, map_id, yards_per_unit, centers[(x, y, spread, n)])
ZONES = [
    ("isle-of-dorn", "khaz-algar", "Isle of Dorn", 2248, 10.0,
     [(30, 35, 9, 26), (62, 55, 11, 30), (48, 72, 8, 24)]),
    ("ringing-deeps", "khaz-algar", "The Ringing Deeps", 2214, 10.0,
     [(42, 30, 10, 30), (65, 60, 9, 28), (30, 68, 8, 22)]),
    ("hallowfall", "khaz-algar", "Hallowfall", 2215, 10.0,
     [(35, 40, 12, 26), (60, 45, 10, 30), (50, 70, 9, 24)]),
    ("azj-kahet", "khaz-algar", "Azj-Kahet", 2255, 10.0,
     [(40, 35, 9, 26), (62, 58, 10, 28), (33, 66, 8, 22)]),
    ("ohnahran-plains", "dragon-isles", "Ohn'ahran Plains", 2023, 10.0,
     [(30, 40, 12, 26), (58, 52, 11, 30), (70, 30, 8, 22)]),
    ("thaldrzsus", "dragon-isles", "Thaldraszus", 2025, 10.0,
     [(38, 42, 10, 26), (60, 60, 10, 28), (50, 28, 8, 22)]),
]

CATEGORIES = [
    ("mining", "Mining"),
    ("herbalism", "Herbalism"),
    ("fishing", "Fishing"),
    ("skinning", "Skinning"),
    ("treasure", "Treasure"),
    ("mob", "Mob farming"),
    ("other", "Other"),
]

# (resource_id, category, name, item_id or None, quality, zones[idx], yield)
RESOURCES = [
    # War Within mining/herbalism (representative sample)
    ("bismuth", "mining", "Bismuth", 161502, 1, [0, 1, 2, 3], 2.0),
    ("ironclaw-ore", "mining", "Ironclaw Ore", 161503, 1, [0, 1, 2, 3], 2.0),
    ("mycobloom", "herbalism", "Mycobloom", 161504, 1, [0, 1, 2, 3], 2.0),
    ("luredrop", "herbalism", "Luredrop", 161505, 1, [1, 2, 3], 2.0),
    # Dragonflight mining/herbalism (representative sample)
    ("serevite-ore", "mining", "Serevite Ore", 161506, 1, [4, 5], 2.0),
    ("draconium-ore", "mining", "Draconium Ore", 161507, 1, [4, 5], 1.0),
    ("hochenblume", "herbalism", "Hochenblume", 161508, 1, [4, 5], 2.0),
    ("writhebark", "herbalism", "Writhebark", 161509, 1, [5], 1.0),
]


def scatter(cx, cy, spread, n):
    pts = []
    for _ in range(n):
        x = min(97.5, max(2.5, RNG.gauss(cx, spread / 2.2)))
        y = min(97.5, max(2.5, RNG.gauss(cy, spread / 2.2)))
        pts.append((round(x, 2), round(y, 2)))
    return pts


def main() -> None:
    zones = [
        {"id": zid, "continent_id": cont, "name": name, "map_id": map_id,
         "yards_per_unit": ypu, "bounds_json": "{}"}
        for zid, cont, name, map_id, ypu, _ in ZONES
    ]
    resources = [
        {"id": rid, "category_id": cat, "key": rid, "name": name,
         "item_id": item, "quality": q}
        for rid, cat, name, item, q, _, _ in RESOURCES
    ]
    observations = []
    for ri, (rid, cat, _name, _item, _q, zone_idxs, base_yield) in enumerate(RESOURCES):
        for zi in zone_idxs:
            zid = ZONES[zi][0]
            map_id = ZONES[zi][3]
            centers = ZONES[zi][5]
            # Rotate centers per resource so resources interleave across the zone.
            pick = centers[(ri + zi) % len(centers)]
            for x, y in scatter(pick[0], pick[1], pick[2], pick[3]):
                qty = base_yield if RNG.random() < 0.7 else base_yield + 1.0
                observations.append({
                    "zone_id": zid, "resource_id": rid, "map_id": map_id,
                    "x": x, "y": y, "yield_qty": qty,
                })
    payload = {
        "generated_by": "scripts/gen-seed.py (seed=20260909)",
        "expansions": EXPANSIONS,
        "continents": CONTINENTS,
        "zones": zones,
        "categories": [{"id": cid, "key": cid, "name": name} for cid, name in CATEGORIES],
        "resources": resources,
        "observations": observations,
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")
    print(f"wrote {OUT.relative_to(ROOT)}: {len(zones)} zones, "
          f"{len(resources)} resources, {len(observations)} observations")


if __name__ == "__main__":
    main()
