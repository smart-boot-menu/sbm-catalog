# AGENTS.md — `sbm-catalog`

Follow the universal Smart Boot Menu agent rules in [`smart-boot-menu/.github/AGENTS.md`](https://github.com/smart-boot-menu/.github/blob/main/AGENTS.md) (locally `../.github/AGENTS.md`).

## Assigned Repository & Role

- **Repository:** `smart-boot-menu/sbm-catalog`
- **Primary Dependencies & Specs:** `sbm-common` (`sbm-schema`), `docs/specifications/05-os-catalog.md`, `docs/threat-model/README.md`

## Responsibilities (What You Own)

- Declarative `OsCatalogEntry` JSON/TOML definitions under `catalog/{linux,bsd,windows,recovery}/` conforming strictly to `sbm-common/schemas/os-catalog-entry.v1.schema.json`.
- Accurate `architectures` and `supported_platforms` tagging (distinguishing `uefi-native` from `apple-silicon-m1n1-uboot`).
- Automated schema validation, mirror checksum verification, TUF signing, and offline mirror bundling tools (`tools/`).

## Strict Boundaries (Do NOT Duplicate or Violate)

ZERO arbitrary executable shell scripts in catalog definitions. Never redistribute proprietary Windows ISOs or Apple firmware.

## Engineering & Performance Targets

100% schema-valid catalog entries with verified SHA-256 digests and HTTPS-only mirrors.

*(Targets are goals, not claims; follow `../docs/testing/benchmarks.md` and archive real measurements in `../research/validation/evidence/`.)*

## Security, Testing & Interface Coordination

- Enforce all invariants in `../docs/threat-model/README.md` (offline-first, no secrets committed, bounded timeouts, safe storage plans).
- Run formatting, clippy, unit tests, and integration hooks before committing.
- Never silently diverge from `sbm-common` types or `docs/specifications/`; follow the Interface Change Protocol in `../.github/agent-reference.md`.
