fn tectonic_before_worldgen(world_root: &std::path::Path) {
    crate::game::native_tectonic_glue::ensure_world_datapack(world_root);
}
