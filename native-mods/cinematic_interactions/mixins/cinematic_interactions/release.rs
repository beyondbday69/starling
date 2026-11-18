// An inventory mouse release: on mobile, Android always sends button: Left (right: false).
// If an RMB drag was in progress (ty = 1) or RMB mode is active, the release must match
// the quick_crafting button (Right) so the quick craft finishes instead of cancelling.
fn cinematic_release(game: &mut crate::game::Game<'_>, right: bool) -> ::core::ops::ControlFlow<()> {
    static IN_RELEASE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if IN_RELEASE.load(std::sync::atomic::Ordering::SeqCst) {
        return ::core::ops::ControlFlow::Continue(());
    }
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    if ci::controls() {
        let is_rmb_drag = game.inv_drag.as_ref().is_some_and(|(ty, _)| *ty == 1);
        if !right && (is_rmb_drag || ci::rmb_active()) {
            IN_RELEASE.store(true, std::sync::atomic::Ordering::SeqCst);
            game.inventory_mouse_release(true);
            IN_RELEASE.store(false, std::sync::atomic::Ordering::SeqCst);
            return ::core::ops::ControlFlow::Break(());
        }
    }
    ::core::ops::ControlFlow::Continue(())
}
