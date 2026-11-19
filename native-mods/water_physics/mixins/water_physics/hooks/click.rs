fn water_physics_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    crate::game::native_water_physics_glue::click(game, mod_id, button);
}
