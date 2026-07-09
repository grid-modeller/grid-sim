#!/usr/bin/env python3
"""Fetch the pinned NESO FES 2025 inputs for the fes-pathway reference.

Downloads into data/packs/fes2025/raw/ and verifies every file against
its pinned sha256 (also recorded in data/packs/fes2025.sha256 and in the
header of data/reference/fes-pathway.toml). Files already present with
the correct checksum are not re-downloaded. Stdlib only.

Licence of everything fetched here: NESO Open Data Licence
(https://www.neso.energy/data-portal/neso-open-licence) — copy, publish,
adapt, redistribute permitted with the attribution "Supported by
National Energy SO Open Data".

NESO versions Data Portal resources in place; if a checksum fails the
table has been revised — diff, record, and re-pin deliberately (see
build.py), never silently.

Usage: python fetch.py <repo-root>
"""

import hashlib
import sys
import urllib.request
from pathlib import Path

FILES = {
    # ES1 electricity supply data table, 2025 edition v006 (capacities).
    "fes2025_es1_v006.csv": (
        "https://api.neso.energy/dataset/549b0667-b533-4748-95bd-f6e13933a47d/"
        "resource/6c78a777-b885-4bb6-bc35-8100f9e137a2/download/fes2025_es1_v006.csv",
        "7b7957443d37a09304fe2877bfa2a7a2fa71f8c00cb9f26308bd58391a4ff805",
    ),
    # ED1 electricity demand summary, 2025 edition v006 (demand_twh).
    "fes2025_ed1_v006.csv": (
        "https://api.neso.energy/dataset/2c15c755-d8fe-4229-9169-3b6dd7c88fec/"
        "resource/300c07b9-baeb-4411-bc40-987cbb4aec0b/download/fes2025_ed1_v006.csv",
        "bd36b16b3f3d0cc5cc8e5118590777d18e3e501a72b325b3de617aa17d15bc24",
    ),
    # FLX1 flexibility data table, 2025 edition v006 (cross-checks only).
    "fes2025_flx1_v006.csv": (
        "https://api.neso.energy/dataset/2e3275e2-dd6a-4c2e-8cfb-eeb9b4320dcb/"
        "resource/299bc6c8-7608-4946-ab0a-5bc22129c897/download/fes2025_flx1_v006.csv",
        "a5049beb77c3a58ba21f975177d309b207c667176a702387b269102fa4594462",
    ),
    # FES 2025 Data Workbook (provenance; ES1 sheet == ES1 CSV values).
    "fes2025-data-workbook.xlsx": (
        "https://www.neso.energy/document/364551/download",
        "f11b9a2c08084d4c4596d67d9e498f46c8739d942ad7dd383580365e10405593",
    ),
    # FES 2025 report PDF (spot-check citations by table/page).
    "fes2025-report.pdf": (
        "https://www.neso.energy/document/364541/download",
        "184c745d74cbf406f3adf5c4b1d73796cfa61d48453414fb30ba4ce3a7172a34",
    ),
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit("usage: python fetch.py <repo-root>")
    raw = Path(sys.argv[1]) / "data" / "packs" / "fes2025" / "raw"
    raw.mkdir(parents=True, exist_ok=True)
    failures = []
    for name, (url, expected) in FILES.items():
        dest = raw / name
        if dest.exists() and sha256(dest) == expected:
            print(f"ok (cached)   {name}")
            continue
        print(f"fetching      {name}")
        req = urllib.request.Request(url, headers={"User-Agent": "grid-sim"})
        with urllib.request.urlopen(req) as resp:
            dest.write_bytes(resp.read())
        got = sha256(dest)
        if got == expected:
            print(f"ok (fetched)  {name}")
        else:
            failures.append((name, expected, got))
            print(f"CHECKSUM MISMATCH {name}\n  expected {expected}\n  got      {got}")
    if failures:
        sys.exit(
            "one or more inputs no longer match their pins — NESO has "
            "revised them. Diff, record the revision, re-pin deliberately."
        )


if __name__ == "__main__":
    main()
