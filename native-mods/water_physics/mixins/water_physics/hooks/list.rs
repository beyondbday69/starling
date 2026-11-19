fn water_physics_list(mods: &mut Vec<crate::mod_api::NativeMod>) {
    crate::game::native_water_physics_glue::list(mods);
}
