SolidCraft Drawing V0.1.17 — HLR V2 removal / renderer retained

This patch restores the pre-V2 HLR implementation from V0.1.16 and removes the V1/V2 selector, audits and experimental classifier. The renderer fixes from V2.0.6 remain: hidden first, visible last, pixel-based dashes, visible-only debug.

INSTALL (from repository root):
1. Back up or commit your current changes.
2. Extract the ZIP over the repo, preserving paths.
3. Run .\remove_hlr_v2.ps1 in PowerShell (or execute its contents manually if script policy blocks it).
4. cargo test -p solidcraft-geom hlr
5. cargo check -p solidcraft-ui-egui
6. cargo run -p solidcraft

NOTE: This overwrites crates/geom/src/hlr/mod.rs with the V0.1.16 baseline. Do not use if you have other local modifications to that file that must be preserved. The script removes only the hlr_v2 module declaration and file. serde_json dependency is left alone because other crates may need it.

Compilation is not verified in this environment.
