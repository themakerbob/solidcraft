SolidCraft Drawing V0.1.28f4a — compile fix

Fixes E0599 in toolbar.rs by replacing three undefined DrawingCommand variants
(DimHorizontal, DimVertical, DimAligned) with the existing SmartDimension command.
Includes the prior V0.1.28f4 circle/arc patch unchanged otherwise.

Extract at SolidCraft repository root.
Run:
  cargo check -p solidcraft-ui-egui
  cargo run -p solidcraft

Not compiled in this environment. gpu.rs is not included.
