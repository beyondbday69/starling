// Touch input handling for Cinematic Interactions:
// Screen-split touch controls with virtual trackpad cursor:
// - Left Half:
//   * Single Tap: Left Click (LMB) — pick up full stack, drop stack, craft result, select recipe.
//   * Double Tap: Pickup-All (Collect All) — gather scattered matching items into cursor stack.
// - Right Half:
//   * Move/Drag: Virtual trackpad aiming — moves '+' crosshair cursor.
//   * Tap: Right Click (RMB) — place 1 item, or split half stack.
//   * Double-Tap & Drag: Real-time RMB Quick-Craft Drag across cells (distribute 1 per slot in line).
//   * Sweep Drag (Option 1): While holding an item, sweeping cursor collects matching items into cursor stack.
// - Direct tap on [ X ]: Cleanly exit workstation.
// - Direct tap on [ ^ ]: Toggle Shift (Quick-Move mode).
// - Long Press (> 350ms): Shift + Click (Quick-Move stack between inventory and table).
fn cinematic_touch(
    game: &mut crate::game::Game<'_>,
    event: &winit::event::WindowEvent,
) -> ::core::ops::ControlFlow<()> {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    use horizon_client::hud::HudBatch;
    use std::sync::Mutex;
    use std::time::Instant;
    use winit::event::{TouchPhase, WindowEvent};
    use winit::keyboard::KeyCode;

    #[derive(Clone, Copy, Debug)]
    struct TouchState {
        id: u64,
        is_left: bool,
        is_lmb_btn: bool,
        is_rmb_btn: bool,
        start_time: Instant,
        last_move_time: Instant,
        start_pos: (f32, f32),
        last_pos: (f32, f32),
        total_motion: f32,
        is_double_tap: bool,
        is_rmb_drag: bool,
        long_press_done: bool,
        last_swept_slot: Option<usize>,
    }

    const EMPTY_TOUCHES: [Option<TouchState>; 4] = [None, None, None, None];
    static TOUCHES: Mutex<[Option<TouchState>; 4]> = Mutex::new(EMPTY_TOUCHES);
    static LAST_TAP_LEFT: Mutex<Option<(Instant, (f32, f32))>> = Mutex::new(None);
    static LAST_TAP_RIGHT: Mutex<Option<(Instant, (f32, f32))>> = Mutex::new(None);

    if !ci::controls() || game.screen != crate::game::Screen::Inventory || ci::walking(Instant::now()) {
        return ::core::ops::ControlFlow::Continue(());
    }

    let WindowEvent::Touch(t) = event else {
        return ::core::ops::ControlFlow::Continue(());
    };

    let [w, h] = game.touch_size();
    let batch = HudBatch::new([w, h]);
    let scale = batch.scale.max(1.0);
    let gui_pos = (t.location.x as f32 / scale, t.location.y as f32 / scale);
    let (bw, bh) = (batch.width, batch.height);
    let now = Instant::now();

    match t.phase {
        TouchPhase::Started => {
            let btns = ci::mobile_button_rects(bw, bh);
            let is_lmb_btn = ci::inside(btns.lmb, gui_pos.0, gui_pos.1);
            let is_rmb_btn = ci::inside(btns.rmb, gui_pos.0, gui_pos.1);

            if is_lmb_btn {
                ci::set_lmb_held(true);
                game.audio.play_button_click();
                game.inventory_press(false);
                ci::set_indicator("LMB: Held (Drag to Take/Move)", [100, 220, 255, 255]);
            } else if is_rmb_btn {
                ci::set_rmb_held(true);
                ci::set_rmb_dragging(true);
                game.audio.play_button_click();
                game.inventory_press(true);
                ci::set_indicator("RMB: Held (Drag to Paint 1)", [255, 215, 60, 255]);
            }

            let is_left = gui_pos.0 < bw * 0.5;

            // Check if this tap started within double-tap window of previous tap on this screen half
            let is_double_tap = if !is_lmb_btn && !is_rmb_btn {
                if is_left {
                    let mut last = LAST_TAP_LEFT.lock().unwrap();
                    if let Some((prev_time, prev_pos)) = *last {
                        let dt = now.duration_since(prev_time).as_millis();
                        let dx = gui_pos.0 - prev_pos.0;
                        let dy = gui_pos.1 - prev_pos.1;
                        let dist = (dx * dx + dy * dy).sqrt();
                        *last = None;
                        dt < 320 && dist < 60.0
                    } else {
                        false
                    }
                } else {
                    let mut last = LAST_TAP_RIGHT.lock().unwrap();
                    if let Some((prev_time, prev_pos)) = *last {
                        let dt = now.duration_since(prev_time).as_millis();
                        let dx = gui_pos.0 - prev_pos.0;
                        let dy = gui_pos.1 - prev_pos.1;
                        let dist = (dx * dx + dy * dy).sqrt();
                        *last = None;
                        dt < 320 && dist < 60.0
                    } else {
                        false
                    }
                }
            } else {
                false
            };

            // Cancel any pending tap if one was scheduled
            ci::cancel_pending_tap();

            let new_state = TouchState {
                id: t.id,
                is_left,
                is_lmb_btn,
                is_rmb_btn,
                start_time: now,
                last_move_time: now,
                start_pos: gui_pos,
                last_pos: gui_pos,
                total_motion: 0.0,
                is_double_tap,
                is_rmb_drag: false,
                long_press_done: false,
                last_swept_slot: None,
            };

            let mut touches = TOUCHES.lock().unwrap();
            let mut stored = false;
            for slot in touches.iter_mut() {
                if let Some(s) = slot {
                    if s.id == t.id {
                        *slot = Some(new_state);
                        stored = true;
                        break;
                    }
                }
            }
            if !stored {
                for slot in touches.iter_mut() {
                    if slot.is_none() {
                        *slot = Some(new_state);
                        stored = true;
                        break;
                    }
                }
            }
            if !stored {
                touches[0] = Some(new_state);
            }

            // Ensure vcursor is initialized
            let cur_vpos = ci::vcursor(bw, bh);
            game.cursor = cur_vpos;
            ::core::ops::ControlFlow::Break(())
        }

        TouchPhase::Moved => {
            let mut touches = TOUCHES.lock().unwrap();
            let mut found_idx = None;
            for (i, slot) in touches.iter().enumerate() {
                if let Some(s) = slot {
                    if s.id == t.id {
                        found_idx = Some(i);
                        break;
                    }
                }
            }
            let Some(slot_idx) = found_idx else {
                return ::core::ops::ControlFlow::Break(());
            };
            let mut state = touches[slot_idx].unwrap();

            let dx = gui_pos.0 - state.last_pos.0;
            let dy = gui_pos.1 - state.last_pos.1;
            let step_dist = (dx * dx + dy * dy).sqrt();
            state.total_motion += step_dist;
            state.last_pos = gui_pos;
            state.last_move_time = now;

            if state.is_lmb_btn || state.is_rmb_btn {
                // Button finger is holding down LMB/RMB on the left side — do not move cursor
            } else if !state.is_left {
                // Right Half of Screen (Cursor Aim + RMB / 1-Item):
                // Touch Moved on Right Half moves the virtual cursor
                const SENSITIVITY: f32 = 1.15;
                let new_cursor = ci::move_vcursor(dx * SENSITIVITY, dy * SENSITIVITY, bw, bh);
                game.cursor = new_cursor;

                // When holding LMB button on left and dragging on right:
                if ci::is_lmb_held() {
                    game.inventory_drag_move();
                } else if ci::is_rmb_held() {
                    // When holding RMB button on left and dragging on right:
                    game.inventory_drag_move();
                } else if game.carried.is_some() && !state.is_rmb_drag {
                    let (slot, outside) = game.inventory_target();
                    if let Some(slot) = slot
                        && !outside
                        && slot != 200
                    {
                        if state.last_swept_slot != Some(slot) {
                            state.last_swept_slot = Some(slot);
                            if let Some(held) = game.carried.clone()
                                && let Some(target) = game.slot_view(slot)
                                && target.item == held.item
                                && held.count < 64
                            {
                                game.menu_click(Some(slot), 0, horizon_client::menu::Input::PickupAll);
                                game.audio.play_button_click();
                                if let Some(key) = game.container {
                                    game.container_changed(key);
                                }
                                ci::set_indicator("Sweep Drag: Collected Items", [90, 200, 255, 255]);
                            }
                        }
                    } else {
                        state.last_swept_slot = None;
                    }
                } else {
                    state.last_swept_slot = None;
                }

                // Double-tap and drag on Right Half starts RMB crafting drag (1 item distributed per slot)
                if state.is_double_tap && !state.is_rmb_drag && state.total_motion > 6.0 {
                    state.is_rmb_drag = true;
                    ci::set_rmb_dragging(true);
                    game.inventory_press(true);
                    ci::set_indicator("RMB Drag: Distributing Items", [255, 215, 60, 255]);
                }

                if state.is_rmb_drag {
                    game.inventory_drag_move();
                }
            }

            // Long press check (> 350ms with little motion on non-button touch): Quick-Move / Shift+Click
            if !state.is_lmb_btn
                && !state.is_rmb_btn
                && !state.long_press_done
                && state.total_motion < 12.0
                && now.duration_since(state.start_time).as_millis() >= 350
            {
                state.long_press_done = true;
                let old_shift = game.keys.contains(&KeyCode::ShiftLeft);
                game.keys.insert(KeyCode::ShiftLeft);
                game.mobile.controls.inventory_shift = true;
                game.inventory_press(false);
                game.inventory_mouse_release(false);
                if !old_shift && !ci::shift_active() {
                    game.keys.remove(&KeyCode::ShiftLeft);
                    game.mobile.controls.inventory_shift = false;
                }
                game.audio.play_button_click();
                if let Some(key) = game.container {
                    game.container_changed(key);
                }
                ci::set_indicator("Quick-Move: Transferred Stack", [255, 235, 120, 255]);
            }

            touches[slot_idx] = Some(state);
            ::core::ops::ControlFlow::Break(())
        }

        TouchPhase::Ended => {
            let mut touches = TOUCHES.lock().unwrap();
            let mut state = None;
            for slot in touches.iter_mut() {
                if let Some(s) = slot {
                    if s.id == t.id {
                        state = slot.take();
                        break;
                    }
                }
            }
            let Some(state) = state else {
                return ::core::ops::ControlFlow::Break(());
            };

            // 0. Finish button holds
            if state.is_lmb_btn {
                ci::set_lmb_held(false);
                game.inventory_mouse_release(false);
                if let Some(key) = game.container {
                    game.container_changed(key);
                }
                return ::core::ops::ControlFlow::Break(());
            }

            if state.is_rmb_btn {
                ci::set_rmb_held(false);
                ci::set_rmb_dragging(false);
                game.inventory_mouse_release(true);
                if let Some(key) = game.container {
                    game.container_changed(key);
                }
                return ::core::ops::ControlFlow::Break(());
            }

            // 1. Finish active RMB drag
            if state.is_rmb_drag {
                ci::set_rmb_dragging(false);
                game.inventory_mouse_release(true);
                if let Some(key) = game.container {
                    game.container_changed(key);
                }
                return ::core::ops::ControlFlow::Break(());
            }

            // 2. If long press already executed, finish
            if state.long_press_done {
                return ::core::ops::ControlFlow::Break(());
            }

            let duration_ms = now.duration_since(state.start_time).as_millis();
            let is_tap = duration_ms < 280 && state.total_motion < 18.0;

            if is_tap {
                let cursor_p = ci::vcursor(bw, bh);
                let touch_p = state.start_pos;
                let exit_rect = ci::exit_button_rect(bw, bh);
                let shift_rect = ci::shift_button_rect(bw, bh);

                // Direct tap on [ X ] (exit) still works in top-right corner
                if ci::inside(exit_rect, touch_p.0, touch_p.1) || ci::inside(exit_rect, cursor_p.0, cursor_p.1) {
                    game.audio.play_button_click();
                    ci::stand_up();
                    ci::reset_vcursor();
                    ci::set_rmb(false);
                    ci::set_rmb_dragging(false);
                    ci::cancel_pending_tap();
                    ci::clear_indicator();
                    *touches = EMPTY_TOUCHES;
                    *LAST_TAP_LEFT.lock().unwrap() = None;
                    *LAST_TAP_RIGHT.lock().unwrap() = None;
                    if game.mobile.ui.modifier_latched(KeyCode::ShiftLeft) {
                        game.mobile.ui.toggle_modifier(KeyCode::ShiftLeft);
                    }
                    game.mobile.controls.inventory_shift = false;
                    game.keys.remove(&KeyCode::ShiftLeft);
                    game.close_container();
                    game.screen = crate::game::Screen::Playing;
                    game.capture(true);
                    game.inv_clicks.skip_release();
                    return ::core::ops::ControlFlow::Break(());
                }

                // Direct tap on [ ^ ] (shift toggle) still works in top-right corner
                if ci::inside(shift_rect, touch_p.0, touch_p.1) || ci::inside(shift_rect, cursor_p.0, cursor_p.1) {
                    game.audio.play_button_click();
                    let on = ci::toggle_shift();
                    game.mobile.controls.inventory_shift = on;
                    if on {
                        if !game.mobile.ui.modifier_latched(KeyCode::ShiftLeft) {
                            game.mobile.ui.toggle_modifier(KeyCode::ShiftLeft);
                        }
                        game.keys.insert(KeyCode::ShiftLeft);
                    } else {
                        if game.mobile.ui.modifier_latched(KeyCode::ShiftLeft) {
                            game.mobile.ui.toggle_modifier(KeyCode::ShiftLeft);
                        }
                        game.keys.remove(&KeyCode::ShiftLeft);
                    }
                    ci::set_indicator(if on { "Shift: ON (Quick-Move)" } else { "Shift: OFF" }, [255, 235, 120, 255]);
                    return ::core::ops::ControlFlow::Break(());
                }

                if state.is_left {
                    // Left Half of Screen (LEFT CLICK - LMB & Pickup All):
                    if state.is_double_tap {
                        // Double Tap on Left Half triggers Pickup-All (Collect All)
                        game.cursor = cursor_p;
                        let (slot, outside) = game.inventory_target();
                        if let Some(slot) = slot && !outside && slot != 200 {
                            game.menu_click(Some(slot), 0, horizon_client::menu::Input::PickupAll);
                            game.audio.play_button_click();
                            if let Some(key) = game.container {
                                game.container_changed(key);
                            }
                        }
                        ci::set_indicator("Pickup All: Collected Scattered Items", [90, 200, 255, 255]);
                    } else {
                        // Single Tap on Left Half triggers Left Click (LMB)
                        game.cursor = cursor_p;
                        game.inventory_press(false);
                        game.inventory_mouse_release(false);
                        game.audio.play_button_click();
                        if let Some(key) = game.container {
                            game.container_changed(key);
                        }
                        ci::set_indicator("Left Click: Picked Up / Placed Stack", [100, 255, 120, 255]);
                        let mut last = LAST_TAP_LEFT.lock().unwrap();
                        *last = Some((now, touch_p));
                    }
                } else {
                    // Right Half of Screen (Cursor Aim + RMB / 1-Item):
                    // Tap on Right Half triggers Right Click (RMB)
                    game.cursor = cursor_p;
                    game.inventory_press(true);
                    game.inventory_mouse_release(true);
                    game.audio.play_button_click();
                    if let Some(key) = game.container {
                        game.container_changed(key);
                    }
                    ci::set_indicator("Right Click: Placed 1 Item / Split", [255, 215, 60, 255]);
                    let mut last = LAST_TAP_RIGHT.lock().unwrap();
                    *last = Some((now, touch_p));
                }
            }

            ::core::ops::ControlFlow::Break(())
        }

        TouchPhase::Cancelled => {
            let mut touches = TOUCHES.lock().unwrap();
            for slot in touches.iter_mut() {
                if let Some(s) = slot {
                    if s.id == t.id {
                        let state = slot.take();
                        if let Some(st) = state {
                            if st.is_lmb_btn {
                                ci::set_lmb_held(false);
                                game.inventory_mouse_release(false);
                            }
                            if st.is_rmb_btn {
                                ci::set_rmb_held(false);
                                ci::set_rmb_dragging(false);
                                game.inventory_mouse_release(true);
                            }
                            if st.is_rmb_drag {
                                ci::set_rmb_dragging(false);
                                game.inventory_mouse_release(true);
                            }
                        }
                        break;
                    }
                }
            }
            ::core::ops::ControlFlow::Break(())
        }
    }
}
