# SolidCraft HLR V2.0.1 — visibility diagnostics

This patch adds **per-piece diagnostics** and a pure visible-over-hidden coincidence policy, with regression tests. It does **not** yet replace V1 visibility classification or connect V2 to Drawing. The Sample Plate.sc fixture is included for subsequent end-to-end integration.

Apply the `crates/` directory over your repository root. The fixture can be kept outside the repo if preferred.

```powershell
cargo test -p solidcraft-geom hlr_v2
cargo test -p solidcraft-geom hlr
```

Next step: wire a model loader to this fixture, identify overlapping projected pieces from the audit, and apply the coincidence policy to *geometrically proven* matches. Do not force all corner edges visible.
