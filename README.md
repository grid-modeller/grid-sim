# grid-sim

A Rust simulator of the GB electricity system, built as a research instrument
and public teaching tool. Two coupled engines:

- **Energy adequacy** — half-hourly chronological dispatch over real weather
  (1985–2024) and real fleet data: storage requirements, curtailment,
  capture prices, system costs.
- **System stability** — swing-equation event simulation: inertia,
  frequency nadir, response adequacy.

Every published number is regenerable: `results = f(scenario file, data-pack
checksum, engine git hash)`. No wall-clock, no unseeded randomness; each
quoted figure is pinned by a regression test against a committed scenario.

## Documentation

Start at [`docs/00-doc-map.md`](docs/00-doc-map.md). The architecture record
(`docs/02-architecture.md`), the domain model, the validation methodology
(against observed 2024 GB outturn), and each stage's acceptance tests are all
in `docs/`.

## Building and testing

```sh
cargo build --workspace          # toolchain pinned in rust-toolchain.toml
cargo test --workspace --lib     # hermetic unit tests — no data needed
```

The full acceptance/regression suite needs the data packs, which are
**fetched and built, never committed** — the repo carries only their
checksum manifests (`data/packs/*.sha256`). See `docs/05-validation.md` and
`scripts/` for the fetch-and-build pipeline, and `data/PROVENANCE.md` for
sources and licences. GB packs rebuild from open NESO/Elexon/Copernicus
sources; the two continental scenarios additionally need a free ENTSO-E
Transparency Platform token.

## Licence

- **Code:** dual-licensed under either the [MIT licence](LICENSE-MIT) or the
  [Apache licence, version 2.0](LICENSE-APACHE), at your option. Unless you
  explicitly state otherwise, any contribution intentionally submitted for
  inclusion in the work by you, as defined in the Apache-2.0 licence, shall
  be dual licensed as above, without any additional terms or conditions.
- **Data:** derived data packs and committed reference data are
  [CC-BY-4.0](data/LICENSE) with upstream attribution requirements — see
  [`data/PROVENANCE.md`](data/PROVENANCE.md).

## Citing

See [`CITATION.cff`](CITATION.cff).
