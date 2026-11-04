from pathlib import Path
import sys
p=Path('crates/ui-egui/src/toolbar.rs')
if not p.is_file():
    sys.exit('Run from the SolidCraft repository root (toolbar.rs not found).')
s=p.read_text(encoding='utf-8')
needle='pub fn shortcuts(app: &mut SolveApp, ctx: &egui::Context) {'
if s.count(needle)!=1: sys.exit('Expected one toolbar::shortcuts function; no changes made.')
fix='''pub fn shortcuts(app: &mut SolveApp, ctx: &egui::Context) {
    // Drawing workspace owns its tool shortcuts. The global Model keymap runs
    // before the Drawing UI, so it must not dispatch Model tools while drawing.
    // This also prevents the Create Sketch dialog from opening on Drawing keys.
    if app.drawing_workspace.active && !crate::render_workspace::active(app) {
        app.viewport.key_hold = None;
        return;
    }
'''
if 'Drawing workspace owns its tool shortcuts' in s:
    print('Already patched; no changes made.')
else:
    p.with_suffix('.rs.h17_backup').write_text(s,encoding='utf-8')
    p.write_text(s.replace(needle,fix,1),encoding='utf-8')
    print('Patched',p,'(backup:',p.with_suffix('.rs.h17_backup'),')')
