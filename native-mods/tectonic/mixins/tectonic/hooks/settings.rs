fn tectonic_settings(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    crate::game::native_tectonic_glue::settings(game, mod_id, buttons);
}
