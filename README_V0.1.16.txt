SolidCraft Drawing V0.1.16 - HLR sampling regression correction

Apply on top of V0.1.15. Replaces crates/geom/src/hlr/mod.rs only.

- Samples spline curves at distinct knot spans, including extremely narrow spans.
- Tests quarter, midpoint and three-quarter positions rather than only midpoint.
- Reports a conservative sampled deviation instead of a raw midpoint estimate.

Run:
  cargo test -p solidcraft-geom hlr
  cargo run -p solidcraft

Not compiled in the patch-generation environment. Please report remaining failures.
