#!/usr/bin/env python3
"""Independent numeric cross-check of core algorithms (see docs/ALGORITHMS.md).

Dependency-free Python reimplementation of key math. Asserts the committed
fixtures/expected/*.json values; Rust tests assert the same numbers, so a
transcription error in either implementation fails loudly (here or in CI).

Float-op order mirrors the Rust code exactly (plain left-fold sums, naive
sqrt(x*x + y*y) instead of hypot, (v-m)*(v-m) instead of **2) so agreement is
expected to ~1e-12; tolerances below only absorb last-ulp platform drift.

Only fixtures with a registered verifier below are accepted — anything else
fails. Verifiers land with their phase (cluster/route/go1: P4).
"""
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXPECTED = ROOT / "fixtures" / "expected"

PASS = 0


def check(name: str, actual: float, expected: float, tol: float = 1e-6) -> None:
    global PASS
    if not math.isclose(actual, expected, rel_tol=tol, abs_tol=tol):
        print(f"FAIL {name}: got {actual!r}, expected {expected!r}")
        sys.exit(1)
    PASS += 1
    print(f"ok  {name} = {actual!r}")


def check_eq(name: str, actual, expected) -> None:
    global PASS
    if actual != expected:
        print(f"FAIL {name}: got {actual!r}, expected {expected!r}")
        sys.exit(1)
    PASS += 1
    print(f"ok  {name} = {actual!r}")


def naive_dist(ax: float, ay: float, bx: float, by: float) -> float:
    dx = ax - bx
    dy = ay - by
    return math.sqrt(dx * dx + dy * dy)


def verify_geo(fx: dict) -> None:
    """Euclidean map distance x yards_per_unit (ALGORITHMS.md §1)."""
    for case in fx["distances"]:
        got = naive_dist(case["a"][0], case["a"][1], case["b"][0], case["b"][1])
        check(f"geo.dist[{case['id']}]", got * case["yards_per_unit"], case["yards"])


# --- clustering (ALGORITHMS.md §§2-3) ----------------------------------------


def node_values(o: dict) -> tuple:
    gross = (o["spawn"] * max(o["yield"], 0.0)) * max(o["unit"], 0)
    modifier = 0.5 + 0.5 * min(max(o["confidence"], 0.0), 1.0)
    return gross, gross * modifier


def diameter_of(members: list, xs: list, ys: list) -> float:
    d = 0.0
    for a in range(len(members)):
        for b in range(a + 1, len(members)):
            i, j = members[a], members[b]
            dd = naive_dist(xs[i], ys[i], xs[j], ys[j])
            if dd > d:
                d = dd
    return d


class UnionFind:
    def __init__(self, cells: list, xs: list, ys: list) -> None:
        self.parent = list(range(len(cells)))
        self.members = [list(c) for c in cells]
        self.dia = [diameter_of(m, xs, ys) for m in self.members]

    def find(self, x: int) -> int:
        while self.parent[x] != x:
            self.parent[x] = self.parent[self.parent[x]]
            x = self.parent[x]
        return x

    def try_union(self, a: int, b: int, max_dia: float, xs: list, ys: list):
        ra, rb = self.find(a), self.find(b)
        if ra == rb:
            return ra
        if len(self.members[ra]) + len(self.members[rb]) > 800:
            return None
        cross = 0.0
        for i in self.members[ra]:
            for j in self.members[rb]:
                dd = naive_dist(xs[i], ys[i], xs[j], ys[j])
                if dd > cross:
                    cross = dd
                if cross > max_dia:
                    return None
        dia = max(self.dia[ra], self.dia[rb], cross)
        if dia > max_dia:
            return None
        la, lb = len(self.members[ra]), len(self.members[rb])
        if la > lb or (la == lb and ra < rb):
            big, small = ra, rb
        else:
            big, small = rb, ra
        self.members[big].extend(self.members[small])
        self.members[small] = []
        self.dia[big] = dia
        self.parent[small] = big
        return big


OFFSETS = [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)]


def py_cluster(observations: list, cfg: dict) -> tuple:
    """Mirror of routes::cluster_observations. Returns (clusters, dropped)."""
    obs = observations[:2000]
    xs = [o["x"] for o in obs]
    ys = [o["y"] for o in obs]
    cells: dict = {}
    for i, o in enumerate(obs):
        key = (math.floor(o["x"] / cfg["cellSize"]), math.floor(o["y"] / cfg["cellSize"]))
        cells.setdefault(key, []).append(i)
    keys = sorted(cells)
    index_of = {k: i for i, k in enumerate(keys)}
    uf = UnionFind([cells[k] for k in keys], xs, ys)
    for i, key in enumerate(keys):
        for dx, dy in OFFSETS:
            j = index_of.get((key[0] + dx, key[1] + dy))
            if j is not None and j > i:
                uf.try_union(i, j, cfg["maxDiameter"], xs, ys)
    groups = []
    for i in range(len(keys)):
        if uf.find(i) == i and uf.members[i]:
            groups.append(sorted(uf.members[i]))
    dropped = 0
    scored = []
    for g in groups:
        if len(g) < cfg["minSamples"]:
            dropped += len(g)
            continue
        value = 0.0
        for i in g:
            value += node_values(obs[i])[1]
        scored.append((value, g))
    scored.sort(key=lambda t: (-t[0], t[1][0]))
    clusters = []
    for n, (value, g) in enumerate(scored):
        sx, sy, gross, conf = 0.0, 0.0, 0.0, 0.0
        members = []
        for i in g:
            gg, ee = node_values(obs[i])
            sx += obs[i]["x"]
            sy += obs[i]["y"]
            gross += gg
            conf += min(max(obs[i]["confidence"], 0.0), 1.0)
            members.append(ee)
        clusters.append(
            {
                "id": f"c{n}",
                "cx": sx / len(g),
                "cy": sy / len(g),
                "members": len(g),
                "gross": gross,
                "effective": value,
                "memberValues": members,
                "confidence": conf / len(g),
            }
        )
    return clusters, dropped


def verify_cluster(fx: dict) -> None:
    for case in fx["cases"]:
        cid = case["id"]
        clusters, dropped = py_cluster(case["observations"], case["config"])
        ex = case["expect"]
        check_eq(f"cluster[{cid}].count", len(clusters), ex["count"])
        check_eq(f"cluster[{cid}].dropped", dropped, ex.get("dropped", 0))
        for i, c in enumerate(clusters):
            check_eq(f"cluster[{cid}].sizes[{i}]", c["members"], ex["sizes"][i])
            check(f"cluster[{cid}].cx[{i}]", c["cx"], ex["centroids"][i][0], 1e-9)
            check(f"cluster[{cid}].cy[{i}]", c["cy"], ex["centroids"][i][1], 1e-9)
            check(f"cluster[{cid}].value[{i}]", c["effective"], ex["values"][i], 1e-6)


# --- routes (ALGORITHMS.md §§4-6) ----------------------------------------------


def fnv_seed(s: str, seed: int) -> int:
    h = 0xCBF29CE484222325 ^ seed
    for byte in s.encode("utf-8"):
        h ^= byte
        h = (h * 0x100000001B3) % (1 << 64)
    return h


def tour_length(order: list, clusters: list, ypu: float) -> float:
    if len(order) < 2:
        return 0.0
    total = 0.0
    for w in range(len(order)):
        a = clusters[order[w]]
        b = clusters[order[(w + 1) % len(order)]]
        total += naive_dist(a["x"], a["y"], b["x"], b["y"]) * ypu
    return total


def py_nn(clusters: list, start: int, ypu: float, seed: int) -> list:
    n = len(clusters)
    visited = [False] * n
    order = []
    current = min(start, n - 1)
    order.append(current)
    visited[current] = True
    while len(order) < n:
        a = clusters[current]
        best = None
        for i, c in enumerate(clusters):
            if visited[i]:
                continue
            d = naive_dist(a["x"], a["y"], c["x"], c["y"]) * ypu
            tie = fnv_seed(c["id"], seed)
            if best is None or d < best[1] or (d == best[1] and tie < best[2]):
                best = (i, d, tie)
        current = best[0]
        visited[current] = True
        order.append(current)
    return order


def py_two_opt(order: list, clusters: list, ypu: float, passes: int) -> list:
    n = len(order)
    if n < 4:
        return list(order)
    best = list(order)
    best_len = tour_length(best, clusters, ypu)
    for _ in range(passes):
        improved = False
        done = False
        for i in range(n - 1):
            if done:
                break
            for j in range(i + 2, n):
                if i == 0 and j == n - 1:
                    continue
                cand = best[: i + 1] + list(reversed(best[i + 1 : j + 1])) + best[j + 1 :]
                leng = tour_length(cand, clusters, ypu)
                if leng < best_len - 1e-9:
                    best, best_len = cand, leng
                    improved = True
                    done = True
                    break
        if not improved:
            break
    return best


def py_simulate(order: list, clusters: list, cfg: dict, ypu: float) -> dict:
    yards = tour_length(order, clusters, ypu)
    nodes, qty, gross, eff, var = 0, 0.0, 0.0, 0.0, 0.0
    for i in order:
        c = clusters[i]
        nodes += c["members"]
        qty += c["qty"]
        gross += c["gross"]
        eff += c["effective"]
        mv = c["memberValues"]
        if len(mv) >= 3 and c["effective"] > 0.0:
            mean = c["effective"] / len(mv)
            v = 0.0
            for x in mv:
                v += (x - mean) * (x - mean)
            v /= len(mv)
            spread = min(max(math.sqrt(v) / mean, 0.15), 0.60)
        else:
            spread = min(max(0.35 + 0.25 * (1.0 - c["confidence"]), 0.15), 0.60)
        t = c["effective"] * spread
        var += t * t
    seconds = yards / max(cfg["travelSpeedYps"], 0.1) + nodes * cfg["gatherSeconds"] \
        + cfg["loopOverheadSeconds"]
    hours = max(seconds / 3600.0, 1e-9)
    std = math.sqrt(var)
    p10 = max(eff - 1.2815515655446004 * std, 0.0)
    p90 = eff + 1.2815515655446004 * std
    return {
        "yards": yards,
        "seconds": seconds,
        "nodes": nodes,
        "qty": qty,
        "gross": gross,
        "effective": eff,
        "gph": eff / hours,
        "conservative": p10 / hours,
        "band": [p10 / hours, eff / hours, p90 / hours],
    }


def verify_route(fx: dict) -> None:
    for case in fx["cases"]:
        cid = case["id"]
        clusters, ypu, cfg = case["clusters"], case["ypu"], case["config"]
        ex = case["expect"]
        nn = py_nn(clusters, case["start"], ypu, cfg["seed"])
        check_eq(f"route[{cid}].nnOrder", nn, ex["nnOrder"])
        init = case.get("initialOrder") or nn
        opt = py_two_opt(init, clusters, ypu, cfg["twoOptPasses"])
        check_eq(f"route[{cid}].optOrder", opt, ex["optOrder"])
        sim = py_simulate(opt, clusters, cfg, ypu)
        check(f"route[{cid}].loopYards", sim["yards"], ex["loopYards"], 1e-9)
        check(f"route[{cid}].loopSeconds", sim["seconds"], ex["loopSeconds"], 1e-9)
        check(f"route[{cid}].gphExpected", sim["gph"], ex["gphExpected"], 1e-6)
        check(f"route[{cid}].gphConservative", sim["conservative"], ex["gphConservative"], 1e-6)
        for k, name in enumerate(["p10", "median", "p90"]):
            check(f"route[{cid}].band.{name}", sim["band"][k], ex["band"][k], 1e-6)


# --- GO1 codec (addon/Bridge.lua + routes.rs) -------------------------------------


def go1_checksum(map_id: int, n: int, coords: str) -> int:
    s = f"{map_id}.{n}.{coords}"
    acc = 0
    for i, ch in enumerate(s, start=1):
        if ch.isdigit():
            acc = (acc + int(ch) * i) % 997
    return acc


def py_go1_encode(map_id: int, wps: list) -> str:
    coords = ""
    for x, y in wps:
        qx = min(max(math.floor(x * 10 + 0.5), 0), 999)
        qy = min(max(math.floor(y * 10 + 0.5), 0), 999)
        coords += f"{qx:03d}{qy:03d}"
    return f"GO1.{map_id}.{len(wps)}.{coords}.{go1_checksum(map_id, len(wps), coords):03d}"


def verify_go1(fx: dict) -> None:
    for i, vec in enumerate(fx["vectors"]):
        got = py_go1_encode(vec["map"], vec["waypoints"])
        check_eq(f"go1[{i}].code", got, vec["code"])


VERIFIERS = {
    "geo.json": verify_geo,
    "cluster.json": verify_cluster,
    "route.json": verify_route,
    "go1.json": verify_go1,
}


def main() -> int:
    files = sorted(EXPECTED.glob("*.json")) if EXPECTED.is_dir() else []
    if not files:
        print("verify-algorithms: no fixtures/expected/*.json — skip")
        return 0
    for path in files:
        fn = VERIFIERS.get(path.name)
        if fn is None:
            print(f"FAIL: fixture {path.name} has no registered verifier")
            return 1
        fn(json.loads(path.read_text(encoding="utf-8")))
    print(f"PASS: {PASS} numeric checks across {len(files)} fixture(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
