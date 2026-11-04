# SolidCraft HLR V2.0.2 — projected coincidence diagnostics

Apply over V2.0.1. Replaces only `crates/geom/src/hlr_v2.rs`. Legacy `hlr` and Drawing remain unchanged.

Adds `coincidence_conflicts(drawing, tolerance_mm)` and `diagnostics_v2_json` for identifying visible/hidden conflicts among genuinely overlapping *straight* projected segments, including source provenance. Four tests cover overlap, crossing, separation and reversed orientation.

This is **not yet** a replacement occlusion classifier, and does not yet decode `Sample Plate.sc` into B-rep; that requires SolidCraft document/kernel cache integration. Curved coincidence and face-depth visibility remain to be implemented.

Run `cargo test -p solidcraft-geom hlr_v2` and `cargo test -p solidcraft-geom hlr`.
