fn water_physics_settings(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    crate::game::native_water_physics_glue::settings(game, mod_id, buttons);
}
