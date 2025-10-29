#!/usr/bin/env python3
import argparse, binascii, zlib, sys
from typing import Tuple, List, Optional

# ---------------------
# Hash-Kandidaten (32-bit)
# ---------------------

def coerce_u32(x: int) -> int:
    return x & 0xFFFFFFFF

def seed_or_default(seed: Optional[int], default: int) -> int:
    return default if seed is None else coerce_u32(seed)

def adler32(data: bytes, seed: Optional[int] = None) -> int:
    # Standard-Default für Adler32 ist 1
    s = seed_or_default(seed, 1)
    return zlib.adler32(data, s) & 0xFFFFFFFF

def crc32(data: bytes, seed: Optional[int] = None) -> int:
    # CRC32-Default ist 0
    s = seed_or_default(seed, 0)
    return binascii.crc32(data, s) & 0xFFFFFFFF

def djb2(data: bytes, seed: Optional[int] = None) -> int:
    s = seed_or_default(seed, 5381)
    h = s
    for b in data:
        h = ((h << 5) + h + b) & 0xFFFFFFFF  # h*33 + b
    return h

def sdbm(data: bytes, seed: Optional[int] = None) -> int:
    s = seed_or_default(seed, 0)
    h = s
    for b in data:
        h = (b + (h << 6) + (h << 16) - h) & 0xFFFFFFFF
    return h

def jenkins_oaat(data: bytes, seed: Optional[int] = None) -> int:
    s = seed_or_default(seed, 0)
    h = s
    for b in data:
        h = (h + b) & 0xFFFFFFFF
        h = (h + (h << 10)) & 0xFFFFFFFF
        h ^= (h >> 6)
    h = (h + (h << 3)) & 0xFFFFFFFF
    h ^= (h >> 11)
    h = (h + (h << 15)) & 0xFFFFFFFF
    return h

# Murmur3 x86_32 (minimal)
def _rotl32(x, r): return ((x << r) | (x >> (32 - r))) & 0xFFFFFFFF
def _fmix32(h):
    h ^= h >> 16; h = (h * 0x85EBCA6B) & 0xFFFFFFFF
    h ^= h >> 13; h = (h * 0xC2B2AE35) & 0xFFFFFFFF
    h ^= h >> 16; return h
def murmur3_x86_32(data: bytes, seed: Optional[int] = None) -> int:
    s = seed_or_default(seed, 0)
    h = s
    c1, c2 = 0xCC9E2D51, 0x1B873593
    n = len(data)
    # body
    i = 0
    while i + 4 <= n:
        k = data[i] | (data[i+1] << 8) | (data[i+2] << 16) | (data[i+3] << 24)
        k = (k * c1) & 0xFFFFFFFF
        k = _rotl32(k, 15)
        k = (k * c2) & 0xFFFFFFFF
        h ^= k
        h = _rotl32(h, 13)
        h = (h * 5 + 0xE6546B64) & 0xFFFFFFFF
        i += 4
    # tail
    k = 0
    tail = data[i:]
    if len(tail) == 3: k ^= tail[2] << 16
    if len(tail) >= 2: k ^= tail[1] << 8
    if len(tail) >= 1:
        k ^= tail[0]
        k = (k * c1) & 0xFFFFFFFF
        k = _rotl32(k, 15)
        k = (k * c2) & 0xFFFFFFFF
        h ^= k
    # finalization
    h ^= n
    return _fmix32(h)

FUNCS = {
    "crc32":       crc32,
    "adler32":     adler32,
    "djb2":        djb2,
    "sdbm":        sdbm,
    "jenkins":     jenkins_oaat,
    "murmur3_32":  murmur3_x86_32,
}

# ---------------------
# Seed-Strategien
# ---------------------

DYN_SEEDS = ["len", "sum8", "sum32"]

def dyn_seed(name: str, payload: bytes) -> int:
    if name == "len":
        return len(payload) & 0xFFFFFFFF
    if name == "sum8":
        return sum(payload) & 0xFF
    if name == "sum32":
        return sum(payload) & 0xFFFFFFFF
    raise KeyError(name)

# Reihenfolge der Seeds: None (Default), feste Werte, dynamische
FIXED_SEEDS = [None, 0, 1, 33, 5381, 0xDEADBEEF, 0xA5A5A5A5]

def all_seed_candidates(payload: bytes) -> List[Tuple[str, Optional[int]]]:
    out = [("fixed", s) for s in FIXED_SEEDS]
    out += [("dyn:"+n, dyn_seed(n, payload)) for n in DYN_SEEDS]
    return out

# ---------------------
# IO / CLI
# ---------------------

def parse_pair(arg: str) -> Tuple[bytes, int, str]:
    """
    Erwartet:  path:0xEXTRA4
    Gibt: (payload_bytes, extra4, path) zurück
    """
    if ":" not in arg:
        raise ValueError("Each argument must be 'file:0xEXTRA4'")
    path, hexv = arg.split(":", 1)
    if not hexv.lower().startswith("0x"):
        raise ValueError("Use hex with 0x prefix for extra4")
    extra4 = int(hexv, 16) & 0xFFFFFFFF
    with open(path, "rb") as f:
        data = f.read()
    return data, extra4, path

def main():
    ap = argparse.ArgumentParser(
        description="Fit EXTRA4 = f(payload) über gängige 32-bit Hashes + Seeds")
    ap.add_argument("pairs", nargs="+",
                    help="payload.bin:0xEXTRA4 (mehrere empfohlen)")
    args = ap.parse_args()

    samples = [parse_pair(p) for p in args.pairs]
    print(f"[info] {len(samples)} samples")

    exact_hits = []
    scored = []  # (mismatches, func, seed_label)

    for fname, fn in FUNCS.items():
        mismatches = 0
        matched_all = False
        # Für jede Funktion eigene Seed-Kombis pro Sample prüfen
        # (gleiche Seed-Strategie muss für ALLE Samples funktionieren!)
        # Wir testen Seeds aus dem ERSTEN Sample (dynamisch/fest),
        # damit dyn Seeds konsistent definiert sind.
        seed_candidates = all_seed_candidates(samples[0][0])
        best_for_func = None

        for label, seed_val in seed_candidates:
            ok = True
            for payload, want, _ in samples:
                got = fn(payload, seed_val)
                if got != want:
                    ok = False
                    break
            if ok:
                exact_hits.append((fname, label, seed_val))
                matched_all = True

        if not matched_all:
            # score: wie viele Samples weichen ab (für Hinweis)
            miss = 0
            for payload, want, _ in samples:
                # nimm z. B. dyn 'len' als Score-Basis
                got = fn(payload, dyn_seed("len", payload))
                if got != want:
                    miss += 1
            scored.append((miss, fname))

    if exact_hits:
        print("[OK] exact matches:")
        for fname, label, seed_val in exact_hits:
            if label.startswith("dyn:"):
                print(f"  {fname:12s} seed={label} (value=0x{seed_val:08x})")
            else:
                sv = "default" if seed_val is None else f"0x{seed_val:08x}"
                print(f"  {fname:12s} seed={sv}")
        sys.exit(0)

    print("[FAIL] no exact match. Try more samples or extend functions/seeds.")
    if scored:
        scored.sort()
        print("Closest (by heuristic mismatches with seed=len):")
        for miss, fname in scored[:8]:
            print(f"  {fname:12s} mismatches={miss}")

if __name__ == "__main__":
    main()
