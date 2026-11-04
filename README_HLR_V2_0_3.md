# HLR V2.0.3 — Live selector

Apply on top of V2.0.2 and Drawing V0.1.16. Includes the Drawing workspace file based on the supplied V0.1.13 ribbon version; back up your local drawing_workspace.rs before overwriting if you have changed it since.

The Drawing workspace now shows a V1 / V2 selector, and the V2 mode shows visibility audit counts per view in the browser. Switching invalidates the projection cache. V1 remains default.

**V2 is NOT an independent visibility engine yet.** It currently delegates projection to V1; the output will be visually identical. The selector is a live integration milestone, not the rounded-corner correction.

Run: cargo check -p solidcraft-ui-egui && cargo run -p solidcraft
