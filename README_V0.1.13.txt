SolidCraft Drawing V0.1.13 - tangent-edge suppression

Apply on top of V0.1.12 (do not revert its HLR coverage patch).
Extract to repository root, overwriting crates/ui-egui/src/drawing_workspace.rs.

Change: drawing projection explicitly sets hlr::Options.tangent_edges=false.
This suppresses smooth fillet/plane face junctions while retaining sharp
edges and analytic silhouettes. The geometry engine remains unchanged.

Check:
  cargo check -p solidcraft-ui-egui
  cargo run -p solidcraft

Verify Front, Top, Right and Isometric views on Sample Plate.
No compile or runtime verification was possible in this environment.
