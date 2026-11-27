// The cinematic bars: two black strips sliding in from the top and the bottom
// of the view while the key is held. They are drawn after the frame's GUI,
// which runs even with the HUD hidden, so they stay while the HUD is away.
fn zoom_bars(game: &crate::game::Game<'_>, batch: &mut crate::hud::HudBatch) {
    use crate::game::official_zoom as zoom;
    if !game.native_on(zoom::ID) || !game.native_playing() {
        return;
    }
    let bar = zoom::bar_height(batch.height);
    if bar <= 0.0 {
        return;
    }
    let atlas = &game.gui.atlas;
    let (width, height) = (batch.width, batch.height);
    batch.fill(atlas, 0.0, 0.0, width, bar, 0xFF00_0000);
    batch.fill(atlas, 0.0, height - bar, width, bar, 0xFF00_0000);
}
