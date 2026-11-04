SolidCraft Drawing V0.1.15 — edge-on rounded-corner HLR coverage

Apply over V0.1.14. Files modified:
 crates/geom/src/hlr/mod.rs
 crates/geom/src/hlr_tests.rs

The V0.1.14 workaround unconditionally preserved any high-sag projected
curve, including hidden rounded-corner arcs that exactly coincide with
visible arcs. Those hidden duplicates were rendered as dotted segments.

V0.1.15 adaptively samples the actual projected curve for coverage and
removes that blanket exemption. Coincident hidden curves can be covered,
while curved edges that genuinely bow away from a straight chord remain.

Tests:
 cargo test -p solidcraft-geom edge_on_corner_hidden_copy_is_covered_but_exposed_arc_is_preserved
 cargo test -p solidcraft-geom hlr
 cargo run -p solidcraft

The ZIP has been integrity-checked. Rust toolchain is unavailable in the
build environment, so compilation and visual confirmation remain pending.
