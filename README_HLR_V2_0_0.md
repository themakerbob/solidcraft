# SolidCraft HLR V2.0.0 — isolated diagnostic foundation

This is the first **non-destructive** HLR V2 deliverable, not a completed replacement HLR algorithm.

## What changes
- Adds `solidcraft_geom::hlr_v2::project_with_audit(...)` as a separate entry point.
- Counts visible/hidden/uncertain and covered pieces, and distinguishes B-rep edges, tangents and silhouettes.
- Adds a unit test for audit initialization.
- Keeps `solidcraft_geom::hlr` unchanged and does not switch the Drawing workspace.

## Apply
Extract at the repository root. This patch only contains `crates/geom/src/lib.rs` and the new `crates/geom/src/hlr_v2.rs`. Note that `lib.rs` is taken from the uploaded `geom.zip`; if your local file has additional modifications, add `pub mod hlr_v2;` to your own file instead of overwriting it.

## Test
```
cargo test -p solidcraft-geom hlr_v2
cargo test -p solidcraft-geom hlr
```

## Next milestones
1. Add an exact Sample Plate fixture and save raw per-piece classification output.
2. Diagnose which corner arc is hidden and which face/ray occludes it.
3. Implement independent V2 visibility resolution, then compare outputs against V1.
4. Switch Drawing to V2 behind a toggle only after regression tests pass.

## Usage inventory
The Drawing workspace (`crates/ui-egui/src/drawing_workspace.rs`) calls `solidcraft_geom::hlr::prepare` and `hidden_lines`. The uploaded `crates.zip` does not show other Rust `hlr::` call sites, but that archive may not include the user's latest local workspace. `hlr` is public in `solidcraft-geom`, so downstream consumers are possible.
