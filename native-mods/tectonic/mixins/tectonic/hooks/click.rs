fn tectonic_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    crate::game::native_tectonic_glue::click(game, mod_id, button);
}
