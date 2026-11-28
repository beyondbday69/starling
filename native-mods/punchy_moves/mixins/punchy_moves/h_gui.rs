// After the frame's GUI: Punchy Moves' screens (the emote wheel, the camera
// page, the colour picker, the face canvas and the tutorial) on the HUD.
fn punchy_moves_gui(game: &crate::game::Game<'_>, batch: &mut crate::hud::HudBatch) {
    use crate::mobs::punchy_moves::{ID, camera, ui};
    if !game.native_on(ID) || !ui::ui(|screens| screens.any_open()) {
        return;
    }
    let layout = ui::UiLayout::new(batch.width, batch.height);
    let settings = camera::settings(|key| game.native_setting(ID, key));
    let frame = ui::ui(|screens| screens.draw(&layout, &settings, &ui::UiTheme::default()));
    let atlas = &game.gui.atlas;
    for draw in frame.draws() {
        match draw {
            ui::HudDraw::Rect(rect, color) => {
                batch.fill(atlas, rect.x, rect.y, rect.w, rect.h, *color);
            }
            ui::HudDraw::Quad(corners, color) => batch.fill_quad(atlas, *corners, *color),
            ui::HudDraw::Text {
                text,
                x,
                y,
                color,
                shadow,
            } => {
                batch.text(atlas, text, *x, *y, *color, *shadow);
            }
        }
    }
}
