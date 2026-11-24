// A click on the book button or in the recipe panel goes to the panel, not to the table behind it.
fn cinematic_press(game: &mut crate::game::Game<'_>, right: bool) -> ::core::ops::ControlFlow<()> {
    static IN_PRESS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if IN_PRESS.load(std::sync::atomic::Ordering::SeqCst) {
        return ::core::ops::ControlFlow::Continue(());
    }

    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    if ci::controls() {
        let is_mobile = cfg!(target_os = "android") || game.touch_shown();
        if is_mobile && let Some(size) = game.renderer.as_ref().map(|r| [r.config.width, r.config.height]) {
            let batch = HudBatch::new(size);
            let (w, h) = (batch.width, batch.height);
            let (cx, cy) = game.cursor;
            let exit_r = ci::exit_button_rect(w, h);
            let shift_r = ci::shift_button_rect(w, h);
            if ci::inside(exit_r, cx, cy) {
                game.audio.play_button_click();
                ci::stand_up();
                ci::reset_vcursor();
                ci::set_rmb(false);
                ci::set_rmb_dragging(false);
                ci::cancel_pending_tap();
                if game.mobile.ui.modifier_latched(winit::keyboard::KeyCode::ShiftLeft) {
                    game.mobile.ui.toggle_modifier(winit::keyboard::KeyCode::ShiftLeft);
                }
                game.mobile.controls.inventory_shift = false;
                game.keys.remove(&winit::keyboard::KeyCode::ShiftLeft);
                game.close_container();
                game.screen = crate::game::Screen::Playing;
                game.capture(true);
                game.inv_clicks.skip_release();
                return ::core::ops::ControlFlow::Break(());
            }
            if ci::inside(shift_r, cx, cy) {
                game.audio.play_button_click();
                let on = ci::toggle_shift();
                game.mobile.controls.inventory_shift = on;
                if on {
                    if !game.mobile.ui.modifier_latched(winit::keyboard::KeyCode::ShiftLeft) {
                        game.mobile.ui.toggle_modifier(winit::keyboard::KeyCode::ShiftLeft);
                    }
                    game.keys.insert(winit::keyboard::KeyCode::ShiftLeft);
                } else {
                    if game.mobile.ui.modifier_latched(winit::keyboard::KeyCode::ShiftLeft) {
                        game.mobile.ui.toggle_modifier(winit::keyboard::KeyCode::ShiftLeft);
                    }
                    game.keys.remove(&winit::keyboard::KeyCode::ShiftLeft);
                }
                game.inv_clicks.skip_release();
                return ::core::ops::ControlFlow::Break(());
            }

            if ci::shift_active() {
                game.mobile.controls.inventory_shift = true;
                if !game.mobile.ui.modifier_latched(winit::keyboard::KeyCode::ShiftLeft) {
                    game.mobile.ui.toggle_modifier(winit::keyboard::KeyCode::ShiftLeft);
                }
                game.keys.insert(winit::keyboard::KeyCode::ShiftLeft);
            } else {
                game.mobile.controls.inventory_shift = false;
                if game.mobile.ui.modifier_latched(winit::keyboard::KeyCode::ShiftLeft) {
                    game.mobile.ui.toggle_modifier(winit::keyboard::KeyCode::ShiftLeft);
                }
                game.keys.remove(&winit::keyboard::KeyCode::ShiftLeft);
            }
        }
    }
    let kind = ci::kind();
    // RMB on the station beside this one: the scene moves over to it (the jar's "move over"): this
    // one closes and the other is used as a click on it would.
    if right
        && ci::controls()
        && let Some(n) = ci::neighbour()
    {
        ci::set_neighbour(None);
        ci::stand_up();
        game.close_container();
        // The use itself runs in the next tick (the game's use of a block is private to its module).
        ci::set_move_to(Some(n));
        game.inv_clicks.skip_release();
        return ::core::ops::ControlFlow::Break(());
    }
    if ci::controls() && ci::is_surface(kind)
        && let Some(size) = game.renderer.as_ref().map(|r| [r.config.width, r.config.height])
    {
        let batch = HudBatch::new(size);
        let (x, y) = game.cursor;
        // The anvil's floating name: a click opens the "New name" box with the name selected; a
        // click anywhere else closes it (the name typed stays).
        if kind == ci::Kind::Anvil && !right {
            if !ci::renaming() && ci::on_name(x, y) {
                if let Some(st) = game.station.as_mut() {
                    let len = st.rename.value.chars().count();
                    st.rename.cursor = len;
                    st.rename.highlight = 0;
                    st.rename.focused = true;
                    st.rename.reset_blink();
                }
                ci::set_renaming(true);
                game.inv_clicks.skip_release();
                return ::core::ops::ControlFlow::Break(());
            }
            if ci::renaming() {
                ci::set_renaming(false);
            }
        }
        if !ci::on_panel(batch.width, x, y) {
            // The campfire: carried food onto a free spot (CampfireStation.click, the game's own
            // use of the food on the fire).
            if kind == ci::Kind::Campfire {
                if !right
                    && ci::surface_hover(ci::ndc((x, y), batch.width, batch.height)).is_some()
                    && let Some(food) = game.carried.clone()
                    && game.recipes().cook(&Default::default(), "minecraft:campfire_cooking", &food).is_some()
                {
                    ci::request_place(ci::table().0);
                }
                game.inv_clicks.skip_release();
                return ::core::ops::ControlFlow::Break(());
            }
            // The chiseled bookshelf: a book into an empty place, or a book taken off a full one
            // (into the inventory), as the block's own use.
            if kind == ci::Kind::Shelf {
                if !right
                    && let Some(i) = ci::surface_hover(ci::ndc((x, y), batch.width, batch.height))
                    && let Some((point, face)) = ci::spot_use(kind, i)
                {
                    let t = ci::table().0;
                    let state = game.world.block(t[0], t[1], t[2]);
                    let key = format!("slot_{i}_occupied");
                    let full = game.pack.assets.registry.state(state).and_then(|s| s.properties.get(&key)).is_some_and(|v| v == "true");
                    let book = game.carried.as_ref().is_some_and(|c| ci::is_book(&c.item));
                    if full {
                        // An empty hand takes: a held book would go in an empty place, not here.
                        let item = match game.world.block_entities.get(&(t[0], t[1], t[2])) {
                            Some(horizon_client::containers::BlockEntity::ChiseledBookshelf(b)) => b.items.get(i).cloned().flatten().map(|s| s.item.to_string()),
                            _ => None,
                        };
                        ci::start_slide(i, false, item.as_deref().unwrap_or("minecraft:book"));
                        ci::request_use(t, point, face, false);
                    } else if book {
                        let item = game.carried.as_ref().map(|c| c.item.to_string()).unwrap_or_default();
                        ci::start_slide(i, true, &item);
                        ci::request_use(t, point, face, true);
                    }
                }
                game.inv_clicks.skip_release();
                return ::core::ops::ControlFlow::Break(());
            }
            // The loom's board: the game's own pattern click.
            if kind == ci::Kind::Loom && ci::on_loom_board(batch.width, batch.height, x, y) {
                let (ox, oy) = ci::loom_board(batch.width, batch.height);
                if !right {
                    game.loom_click((x - ox, y - oy));
                }
                game.inv_clicks.skip_release();
                return ::core::ops::ControlFlow::Break(());
            }
            // A rune or a power: the station's own click.
            let rune = kind == ci::Kind::Enchanting && ci::rune_at(x, y, batch.width, batch.height).is_some();
            let power = kind == ci::Kind::Beacon && ci::power_at(x, y, batch.width, batch.height).is_some();
            if !right && (rune || power) {
                game.station_click(false);
                game.inv_clicks.skip_release();
                return ::core::ops::ControlFlow::Break(());
            }
            // The payment, clicked with an empty hand once a power is chosen: light the beacon.
            let paid = game.station.as_ref().is_some_and(|s| s.inputs.first().is_some_and(|i| i.is_some()) && s.beacon[0].is_some());
            if !right && kind == ci::Kind::Beacon && paid && game.carried.is_none() && ci::surface_hover(ci::ndc((x, y), batch.width, batch.height)) == Some(0) {
                game.beacon_click(ci::BEACON_CONFIRM);
                ci::start_burst(kind);
                game.inv_clicks.skip_release();
                return ::core::ops::ControlFlow::Break(());
            }
        }
        return ::core::ops::ControlFlow::Continue(());
    }
    if !ci::controls() || !ci::table_scene() {
        return ::core::ops::ControlFlow::Continue(());
    }
    let Some(size) = game.renderer.as_ref().map(|r| [r.config.width, r.config.height]) else {
        return ::core::ops::ControlFlow::Continue(());
    };
    let batch = HudBatch::new(size);
    let (x, y) = game.cursor;
    let on_book = if ci::book_open() { ci::book_hit(x, y, ci::selected().is_some()).is_some() } else { ci::on_recipe(batch.width, batch.height, x, y) };
    if game.recipe_book_click(right) || on_book {
        game.inv_clicks.skip_release();
        return ::core::ops::ControlFlow::Break(());
    }
    ::core::ops::ControlFlow::Continue(())
}
