fn water_physics_frame(game: &mut crate::game::Game<'_>) {
    crate::game::native_water_physics_glue::frame(game);
}
