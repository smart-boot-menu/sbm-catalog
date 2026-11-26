# Operating System Catalog & Verified Recipes (`sbm-catalog`)

TUF-signed declarative operating system catalog, verified HTTPS mirror definitions, platform compatibility metadata, and validation tooling.

> **Project Status:** Phase 0 initialized — ready for dedicated engineering agent implementation targeting **SBM 0.1**.

## Scope & Responsibilities

- Declarative `OsCatalogEntry` JSON/TOML definitions under `catalog/{linux,bsd,windows,recovery}/` conforming strictly to `sbm-common/schemas/os-catalog-entry.v1.schema.json`.
- Accurate `architectures` and `supported_platforms` tagging (distinguishing `uefi-native` from `apple-silicon-m1n1-uboot`).
- Automated schema validation, mirror checksum verification, TUF signing, and offline mirror bundling tools (`tools/`).

## Canonical Documentation & Contracts

- Central coordination & roadmap: [`smart-boot-menu/sbm`](https://github.com/smart-boot-menu/sbm)
- Normative specifications & ADRs: [`smart-boot-menu/docs`](https://github.com/smart-boot-menu/docs)
- Primary-source technical research: [`smart-boot-menu/research`](https://github.com/smart-boot-menu/research)
- Shared Rust types & JSON Schemas: [`smart-boot-menu/sbm-common`](https://github.com/smart-boot-menu/sbm-common)
- Agent guide: [AGENTS.md](AGENTS.md)

## License

Original code and documentation in this repository are dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE), subject to third-party licensing requirements for bundled external components.
