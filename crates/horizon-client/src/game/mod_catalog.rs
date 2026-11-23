//! The in-game Resource Pack, Shader Pack, and Mod downloader / catalog.
//! Allows players to search, view icons, read descriptions, browse all available
//! versions, and install directly into `resourcepacks/` or `shaderpacks/` from
//! within Minecraft without leaving the game or using an external launcher.
use super::Game;
use super::native_builds::{density, wrapped};
use horizon_client::{
    font::Patch,
    hud::{HudBatch, PACK_ICON_SIZE, PACK_ICON_SLOTS},
    ui_theme::{Face, Ui},
};
use starling_catalog::{
    Done, Shelf,
    browser::{Kind, Project, Version, installed::Entry},
    list::{self, Ask, Feed, Scroller, Tail},
};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant};
use winit::event::{ElementState, KeyEvent, Touch, TouchPhase};
use winit::keyboard::{Key, NamedKey};

const SHELVES: [(Shelf, &str); 3] = [
    (Shelf::ResourcePacks, "Resource Packs"),
    (Shelf::ShaderPacks, "Shader Packs"),
    (Shelf::Mods, "Mods"),
];

// In UI units: a landscape phone's short side is 700 of them (`density`), about 0.1 mm each.
const MARGIN: f32 = 16.0;
/// What's above and below the list.
const HEAD_H: f32 = 158.0;
const FOOT_H: f32 = 78.0;

/// A row and the gap under it: accommodates the 108x108 pack icon preview and 3-line info block.
const ROW_H: f32 = 148.0;
const ROW_GAP: f32 = 12.0;
const ICON_SIZE: f32 = 108.0;

/// Action buttons in each row.
const ROW_BUTTON_W: f32 = 180.0;
const ROW_BUTTON_H: f32 = 52.0;
const VER_BUTTON_H: f32 = 46.0;

/// The line at the end of the list.
const END_H: f32 = 96.0;
const CLEAR_W: f32 = 96.0;

/// Text sizes:
const TITLE_TEXT: f32 = 25.0;
const META_TEXT: f32 = 17.0;
const BODY_TEXT: f32 = 20.0;
const BUTTON_TEXT: f32 = 20.0;
/// A line of `BODY_TEXT` (`Ui::text`).
const BODY_LINE: f32 = 28.0;

/// The next page is asked for when the end of the list is this close to what shows.
const NEAR_END: f32 = 3.0 * ROW_H;
const NAV_BAR_SHARE: f32 = 0.08;
const QUERY_CHARS: usize = 64;
const TYPING_PAUSE: Duration = Duration::from_millis(400);
const FILE_CHECK: Duration = Duration::from_secs(2);

const CONFIRM_W: f32 = 640.0;
const CONFIRM_H: f32 = 270.0;

#[derive(Clone, PartialEq)]
enum Act {
    Close,
    Shelf(Shelf),
    Search,
    Clear,
    Everything,
    Retry,
    Install(usize),
    ViewVersions(usize),
    InstallVersion(usize),
    ToggleVersionsFilter,
    CloseVersions,
    Remove(usize),
    ConfirmRemove,
    CancelRemove,
}

#[derive(Default)]
struct Shared {
    feed: Feed,
    installing: Option<String>,
    line: String,
    fraction: f32,
    finished: Option<(String, Result<Done, String>)>,
}

pub(super) struct Catalog {
    pub(super) shown: bool,
    shelf: Shelf,
    query: String,
    typing: bool,
    typed_at: Option<Instant>,
    everything: bool,
    list: Scroller,
    installed: HashSet<String>,
    checked_at: Option<Instant>,
    removing: Option<Project>,
    notice: Option<(String, bool)>,
    shared: Arc<Mutex<Shared>>,
    buttons: RefCell<Vec<([f32; 4], Act)>>,
    // Async icon loading and memory cache
    icons: HashMap<String, Arc<[u8]>>,
    loading_icons: HashSet<String>,
    icon_rx: Option<mpsc::Receiver<(String, Arc<[u8]>)>>,
    icon_tx: mpsc::Sender<(String, Arc<[u8]>)>,
    // "All Versions" modal state
    versions_project: Option<Project>,
    versions_list: Vec<Version>,
    versions_loading: bool,
    versions_error: Option<String>,
    versions_scroller: Scroller,
    versions_filter_current: bool,
    versions_rx: Option<mpsc::Receiver<(String, Result<Vec<Version>, String>)>>,
    versions_tx: mpsc::Sender<(String, Result<Vec<Version>, String>)>,
}

impl Default for Catalog {
    fn default() -> Self {
        let (icon_tx, icon_rx) = mpsc::channel();
        let (versions_tx, versions_rx) = mpsc::channel();
        Self {
            shown: false,
            shelf: Shelf::ResourcePacks,
            query: String::new(),
            typing: false,
            typed_at: None,
            everything: false,
            list: Scroller::default(),
            installed: HashSet::new(),
            checked_at: None,
            removing: None,
            notice: None,
            shared: Arc::new(Mutex::new(Shared::default())),
            buttons: RefCell::new(Vec::new()),
            icons: HashMap::new(),
            loading_icons: HashSet::new(),
            icon_rx: Some(icon_rx),
            icon_tx,
            versions_project: None,
            versions_list: Vec::new(),
            versions_loading: false,
            versions_error: None,
            versions_scroller: Scroller::default(),
            versions_filter_current: false,
            versions_rx: Some(versions_rx),
            versions_tx,
        }
    }
}

fn format_downloads(downloads: u64) -> String {
    if downloads >= 1_000_000 {
        format!("{:.1}M", downloads as f64 / 1_000_000.0)
    } else if downloads >= 1_000 {
        format!("{:.1}K", downloads as f64 / 1_000.0)
    } else {
        downloads.to_string()
    }
}

fn format_categories(project: &Project) -> String {
    let list: Vec<&str> = project
        .loaders
        .iter()
        .chain(project.game_versions.iter())
        .take(3)
        .map(String::as_str)
        .collect();
    if list.is_empty() {
        String::new()
    } else {
        format!(" • {}", list.join(", "))
    }
}

/// `text` on one line that fits `width`, cut with dots if it must be.
fn fitted(ui: &Ui<'_>, face: Face, size: f32, width: f32, text: &str) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if ui.measure(face, size, &text) <= width {
        return text;
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let cut = format!("{}...", chars.iter().collect::<String>().trim_end());
        if ui.measure(face, size, &cut) <= width {
            return cut;
        }
    }
    String::new()
}

/// `text` in at most `most` lines that fit `width`, the last cut with dots if it must be.
fn fitted_lines(
    ui: &Ui<'_>,
    face: Face,
    size: f32,
    width: f32,
    most: usize,
    text: &str,
) -> Vec<String> {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut lines = wrapped(ui, face, size, width, &text);
    if lines.len() > most {
        let rest = lines.split_off(most - 1).join(" ");
        lines.push(rest);
    }
    lines
        .iter()
        .map(|line| fitted(ui, face, size, width, line))
        .collect()
}

/// Authentic Minecraft styling and color palette (no launcher purple/black theme).
mod mc {
    pub const fn hex(c: u32, a: u8) -> u32 {
        (a as u32) << 24 | (c & 0xff) << 16 | (c >> 8 & 0xff) << 8 | (c >> 16 & 0xff)
    }

    // Screen backdrops and header/footer bars
    pub const SCREEN_BG: u32 = hex(0x101010, 215);        // In-world dark translucent overlay
    pub const HEADER_BG: u32 = hex(0x181818, 250);        // Minecraft solid charcoal header/footer
    pub const SEPARATOR_DARK: u32 = hex(0x000000, 255);   // 1px black groove border
    pub const SEPARATOR_LIGHT: u32 = hex(0x3e3e3e, 255);  // 1px gray groove border

    // Slot rows
    pub const SLOT_BG: u32 = hex(0x181818, 235);
    pub const SLOT_BG_HOVER: u32 = hex(0x222222, 250);
    pub const SLOT_BORDER_BLACK: u32 = hex(0x000000, 255);
    pub const SLOT_BORDER_IDLE: u32 = hex(0x2d2d2d, 255);
    pub const SLOT_BORDER_HOVER: u32 = hex(0xffffff, 255);

    // Beveled Minecraft buttons
    pub const BTN_BORDER_BLACK: u32 = hex(0x000000, 255);
    pub const BTN_HI_IDLE: u32 = hex(0x8b8b8b, 255);      // Inner top/left highlight
    pub const BTN_SH_IDLE: u32 = hex(0x373737, 255);      // Inner bottom/right shadow
    pub const BTN_FILL_IDLE: u32 = hex(0x4c4c4c, 255);    // Standard Minecraft button fill
    pub const BTN_HI_HOVER: u32 = hex(0xffffff, 255);     // Inner top/left highlight on hover
    pub const BTN_SH_HOVER: u32 = hex(0x555555, 255);     // Inner bottom/right shadow on hover
    pub const BTN_FILL_HOVER: u32 = hex(0x6e6e6e, 255);   // Lighter gray fill on hover/selected
    pub const BTN_FILL_DISABLED: u32 = hex(0x282828, 255);

    // Minecraft EditBox
    pub const EDITBOX_BG: u32 = hex(0x000000, 255);
    pub const EDITBOX_BORDER_IDLE: u32 = hex(0x757575, 255);
    pub const EDITBOX_BORDER_FOCUSED: u32 = hex(0xffffff, 255);

    // Typography
    pub const TEXT_WHITE: u32 = hex(0xffffff, 255);
    pub const TEXT_YELLOW: u32 = hex(0xffffa0, 255);
    pub const TEXT_GOLD: u32 = hex(0xffaa00, 255);
    pub const TEXT_GRAY: u32 = hex(0xaaaaaa, 255);
    pub const TEXT_DARK_GRAY: u32 = hex(0x555555, 255);
    pub const TEXT_GREEN: u32 = hex(0x55ff55, 255);
    pub const TEXT_RED: u32 = hex(0xff5555, 255);
    pub const TEXT_SHADOW: u32 = hex(0x181818, 255);

    // Modals
    pub const MODAL_DIM: u32 = hex(0x000000, 215);
    pub const MODAL_BG: u32 = hex(0x222222, 255);
    pub const MODAL_BORDER_LIGHT: u32 = hex(0x8b8b8b, 255);
    pub const MODAL_BORDER_DARK: u32 = hex(0x373737, 255);

    // Progress
    pub const PROGRESS_BG: u32 = hex(0x202020, 255);
    pub const PROGRESS_GREEN: u32 = hex(0x55ff55, 255);
}

fn mc_is_hovered(ui: &Ui<'_>, rect: [f32; 4]) -> bool {
    let (cx, cy) = ui.cursor;
    cx >= rect[0] && cx <= rect[0] + rect[2] && cy >= rect[1] && cy <= rect[1] + rect[3]
}

fn mc_text_shadow(ui: &mut Ui<'_>, face: Face, size: f32, x: f32, y: f32, color: u32, text: &str) {
    ui.text(face, size, x + 1.5, y + 1.5, mc::TEXT_SHADOW, text);
    ui.text(face, size, x, y, color, text);
}

fn mc_text_line_shadow(
    ui: &mut Ui<'_>,
    face: Face,
    size: f32,
    line: f32,
    x: f32,
    y: f32,
    color: u32,
    text: &str,
) -> f32 {
    ui.text_line(face, size, line, x + 1.5, y + 1.5, mc::TEXT_SHADOW, text);
    ui.text_line(face, size, line, x, y, color, text)
}

fn mc_label_shadow(ui: &mut Ui<'_>, rect: [f32; 4], text: &str, color: u32) {
    let width = ui.measure(Face::Semibold, BUTTON_TEXT, text);
    let x = rect[0] + (rect[2] - width) / 2.0;
    mc_text_line_shadow(
        ui,
        Face::Semibold,
        BUTTON_TEXT,
        rect[3],
        x,
        rect[1],
        color,
        text,
    );
}

/// Authentic 3D beveled Minecraft button.
fn mc_button(
    ui: &mut Ui<'_>,
    rect: [f32; 4],
    text: &str,
    lit: bool,
    enabled: bool,
    act: Option<Act>,
    drawn: &mut Vec<([f32; 4], Act)>,
) {
    let hovered = mc_is_hovered(ui, rect);
    let (x, y, w, h) = (rect[0], rect[1], rect[2], rect[3]);

    // 1px black outer border
    ui.fill(x, y, w, h, mc::BTN_BORDER_BLACK);

    if !enabled {
        ui.fill(x + 1.0, y + 1.0, w - 2.0, 1.0, mc::BTN_SH_IDLE);
        ui.fill(x + 1.0, y + 1.0, 1.0, h - 2.0, mc::BTN_SH_IDLE);
        ui.fill(x + w - 2.0, y + 1.0, 1.0, h - 2.0, mc::BTN_SH_IDLE);
        ui.fill(x + 1.0, y + h - 2.0, w - 2.0, 1.0, mc::BTN_SH_IDLE);
        ui.fill(x + 2.0, y + 2.0, w - 4.0, h - 4.0, mc::BTN_FILL_DISABLED);
        mc_label_shadow(ui, rect, text, mc::TEXT_DARK_GRAY);
        return;
    }

    let active = hovered || lit;
    let hi_color = if active { mc::BTN_HI_HOVER } else { mc::BTN_HI_IDLE };
    let sh_color = if active { mc::BTN_SH_HOVER } else { mc::BTN_SH_IDLE };
    let fill_color = if active { mc::BTN_FILL_HOVER } else { mc::BTN_FILL_IDLE };
    let text_color = if active { mc::TEXT_YELLOW } else { mc::TEXT_WHITE };

    // 1px top highlight
    ui.fill(x + 1.0, y + 1.0, w - 2.0, 1.0, hi_color);
    // 1px left highlight
    ui.fill(x + 1.0, y + 1.0, 1.0, h - 2.0, hi_color);
    // 1px right shadow
    ui.fill(x + w - 2.0, y + 1.0, 1.0, h - 2.0, sh_color);
    // 1px bottom shadow
    ui.fill(x + 1.0, y + h - 2.0, w - 2.0, 1.0, sh_color);
    // Center fill
    ui.fill(x + 2.0, y + 2.0, w - 4.0, h - 4.0, fill_color);

    mc_label_shadow(ui, rect, text, text_color);

    if let Some(act) = act {
        drawn.push((rect, act));
    }
}

/// `text` in the middle of `rect`.
fn label(ui: &mut Ui<'_>, rect: [f32; 4], ink: u32, text: &str) {
    let width = ui.measure(Face::Semibold, BUTTON_TEXT, text);
    let x = rect[0] + (rect[2] - width) / 2.0;
    mc_text_line_shadow(
        ui,
        Face::Semibold,
        BUTTON_TEXT,
        rect[3],
        x,
        rect[1],
        ink,
        text,
    );
}

/// A button's look: styled as Minecraft button.
fn plate(ui: &mut Ui<'_>, rect: [f32; 4], text: &str, lit: bool) {
    let mut unused = Vec::new();
    mc_button(ui, rect, text, lit, true, None, &mut unused);
}

/// A button above or below the list.
fn button(
    ui: &mut Ui<'_>,
    rect: [f32; 4],
    text: &str,
    lit: bool,
    act: Act,
    drawn: &mut Vec<([f32; 4], Act)>,
) {
    mc_button(ui, rect, text, lit, true, Some(act), drawn);
}

/// What to do next, after an install.
fn done_notice(done: Done, title: &str) -> String {
    match done {
        Done::NativeMod => {
            format!("{title} is in. Close this and press Compile on its row to play with it.")
        }
        Done::ContentMod => format!("{title} is in. It loads the next time you start the game."),
        Done::ResourcePack => format!("{title} is in. Turn it on in Options > Resource Packs."),
        Done::ShaderPack => format!("{title} is in. Choose it on the Shader Packs page."),
    }
}

/// The list's height with `rows` rows and its last line.
fn content_h(rows: usize) -> f32 {
    rows as f32 * ROW_H + END_H
}

/// What the last line says while a page is on its way.
fn loading_line(shelf: Shelf, first: bool) -> String {
    let what = match shelf {
        Shelf::Mods => "mods",
        Shelf::ResourcePacks => "resource packs",
        Shelf::ShaderPacks => "shader packs",
    };
    if first {
        format!("Loading {what}...")
    } else {
        format!("Loading more {what}...")
    }
}

/// An installed row's button area: the word Installed over a Remove button.
fn installed_parts(rect: [f32; 4]) -> ([f32; 4], [f32; 4]) {
    let state = [rect[0], rect[1] - 14.0, rect[2], 30.0];
    let remove = [rect[0], rect[1] + 20.0, rect[2], rect[3] - 10.0];
    (state, remove)
}

/// The question over everything while the player is asked about deleting a file, and its buttons
fn draw_confirm(ui: &mut Ui<'_>, title: &str, drawn: &mut Vec<([f32; 4], Act)>) {
    drawn.clear();
    let (w, h) = (ui.w, ui.h);
    ui.fill(0.0, 0.0, w, h, mc::MODAL_DIM);
    let width = CONFIRM_W.min(w - 2.0 * MARGIN);
    let panel = [(w - width) / 2.0, (h - CONFIRM_H) / 2.0, width, CONFIRM_H];

    // Minecraft dialog frame
    let (px, py, pw, ph) = (panel[0], panel[1], panel[2], panel[3]);
    ui.fill(px, py, pw, ph, mc::BTN_BORDER_BLACK);
    ui.fill(px + 1.0, py + 1.0, pw - 2.0, 1.0, mc::MODAL_BORDER_LIGHT);
    ui.fill(px + 1.0, py + 1.0, 1.0, ph - 2.0, mc::MODAL_BORDER_LIGHT);
    ui.fill(px + pw - 2.0, py + 1.0, 1.0, ph - 2.0, mc::MODAL_BORDER_DARK);
    ui.fill(px + 1.0, py + ph - 2.0, pw - 2.0, 1.0, mc::MODAL_BORDER_DARK);
    ui.fill(px + 2.0, py + 2.0, pw - 4.0, ph - 4.0, mc::MODAL_BG);

    let inner = width - 48.0;
    let head = fitted(
        ui,
        Face::Semibold,
        TITLE_TEXT,
        inner,
        &format!("Remove {title}?"),
    );
    mc_text_shadow(
        ui,
        Face::Semibold,
        TITLE_TEXT,
        panel[0] + 24.0,
        panel[1] + 20.0,
        mc::TEXT_YELLOW,
        &head,
    );
    let body = "This deletes its file from your game folder. You can install it again from here.";
    let mut y = panel[1] + 70.0;
    for line in fitted_lines(ui, Face::Regular, BODY_TEXT, inner, 3, body) {
        mc_text_shadow(
            ui,
            Face::Regular,
            BODY_TEXT,
            panel[0] + 24.0,
            y,
            mc::TEXT_GRAY,
            &line,
        );
        y += BODY_LINE;
    }
    let by = panel[1] + CONFIRM_H - ROW_BUTTON_H - 20.0;
    let remove = [
        panel[0] + width - 2.0 * ROW_BUTTON_W - 36.0,
        by,
        ROW_BUTTON_W,
        ROW_BUTTON_H,
    ];
    let cancel = [
        panel[0] + width - ROW_BUTTON_W - 24.0,
        by,
        ROW_BUTTON_W,
        ROW_BUTTON_H,
    ];
    mc_button(ui, remove, "Remove", false, true, Some(Act::ConfirmRemove), drawn);
    mc_button(ui, cancel, "Cancel", false, true, Some(Act::CancelRemove), drawn);
}

/// A search that failed: the log has what it said, the screen one plain sentence.
fn search_failed(raw: &str) -> String {
    log::warn!("Horizon: the catalog's search failed: {raw}");
    list::friendly(raw, "The list didn't load. Try again.")
}

/// An install that failed, the same way.
fn install_failed(title: &str, raw: &str) -> String {
    log::warn!("Horizon: the catalog could not install {title}: {raw}");
    list::friendly(raw, &format!("Couldn't install {title}. Try again."))
}

/// A removal that failed, the same way.
fn remove_failed(title: &str, raw: &str) -> String {
    log::warn!("Horizon: the catalog could not remove {title}: {raw}");
    list::friendly(raw, &format!("Couldn't remove {title}. Try again."))
}

/// The rows that show and the list's last line, under what's above and below them.
fn draw_list(
    ui: &mut Ui<'_>,
    card: &Catalog,
    shared: &Shared,
    edge: f32,
    drawn: &mut Vec<([f32; 4], Act)>,
) {
    let (top, bottom) = (HEAD_H, ui.h - FOOT_H);
    let row_w = edge - MARGIN;
    let rows = shared.feed.rows();
    let scrolled = card.list.offset;
    for i in list::rows_shown(scrolled, bottom - top, ROW_H, rows.len()) {
        let project = &rows[i];
        let y = top + i as f32 * ROW_H - scrolled;
        let row = ROW_H - ROW_GAP;
        let row_rect = [MARGIN, y, row_w, row];
        let row_hovered = mc_is_hovered(ui, row_rect);

        // Minecraft slot box
        let border_color = if row_hovered {
            mc::SLOT_BORDER_HOVER
        } else {
            mc::SLOT_BORDER_IDLE
        };
        let bg_color = if row_hovered {
            mc::SLOT_BG_HOVER
        } else {
            mc::SLOT_BG
        };
        ui.fill(MARGIN, y, row_w, row, mc::SLOT_BORDER_BLACK);
        ui.fill(MARGIN + 1.0, y + 1.0, row_w - 2.0, 1.0, border_color);
        ui.fill(MARGIN + 1.0, y + 1.0, 1.0, row - 2.0, border_color);
        ui.fill(MARGIN + row_w - 2.0, y + 1.0, 1.0, row - 2.0, border_color);
        ui.fill(MARGIN + 1.0, y + row - 2.0, row_w - 2.0, 1.0, border_color);
        ui.fill(MARGIN + 2.0, y + 2.0, row_w - 4.0, row - 4.0, bg_color);

        // 1. Pack Preview Icon
        let icon_x = MARGIN + 12.0;
        let icon_y = y + (row - ICON_SIZE) / 2.0;
        ui.fill(icon_x, icon_y, ICON_SIZE, ICON_SIZE, mc::BTN_BORDER_BLACK);
        ui.fill(icon_x + 1.0, icon_y + 1.0, ICON_SIZE - 2.0, 1.0, mc::BTN_SH_IDLE);
        ui.fill(icon_x + 1.0, icon_y + 1.0, 1.0, ICON_SIZE - 2.0, mc::BTN_SH_IDLE);
        ui.fill(icon_x + ICON_SIZE - 2.0, icon_y + 1.0, 1.0, ICON_SIZE - 2.0, mc::BTN_SH_IDLE);
        ui.fill(icon_x + 1.0, icon_y + ICON_SIZE - 2.0, ICON_SIZE - 2.0, 1.0, mc::BTN_SH_IDLE);
        ui.fill(icon_x + 2.0, icon_y + 2.0, ICON_SIZE - 4.0, ICON_SIZE - 4.0, mc::hex(0x0e0e0e, 255));

        let mut drawn_icon = false;
        if let Some(rgba) = card.icons.get(&project.id) {
            let slot = i % PACK_ICON_SLOTS;
            let sprite_name = format!("horizon/pack-icon{slot}");
            if let Some(rect) = ui.atlas.sprites.get(&sprite_name).copied() {
                let patch = Patch {
                    x: (rect.u0 * ui.atlas.width as f32).round() as u32,
                    y: (rect.v0 * ui.atlas.height as f32).round() as u32,
                    width: PACK_ICON_SIZE,
                    height: PACK_ICON_SIZE,
                    rgba: rgba.to_vec(),
                };
                ui.batch.patches.push(patch);
                ui.blit_sprite(icon_x + 2.0, icon_y + 2.0, ICON_SIZE - 4.0, ICON_SIZE - 4.0, rect);
                drawn_icon = true;
            }
        }
        if !drawn_icon {
            if let Some(rect) = ui
                .atlas
                .sprites
                .get("textures/misc/unknown_pack.png")
                .copied()
            {
                ui.blit_sprite(
                    icon_x + 18.0,
                    icon_y + 18.0,
                    ICON_SIZE - 36.0,
                    ICON_SIZE - 36.0,
                    rect,
                );
            }
        }

        // 2. Title, Author, Downloads, Categories & Description
        let text_x = icon_x + ICON_SIZE + 16.0;
        let text_w = row_w - (text_x - MARGIN) - ROW_BUTTON_W - 24.0;
        let title = if starling_catalog::is_native_mod(project) {
            format!("{} (native mod)", project.title)
        } else {
            project.title.clone()
        };
        let title = fitted(ui, Face::Semibold, TITLE_TEXT, text_w, &title);
        mc_text_shadow(
            ui,
            Face::Semibold,
            TITLE_TEXT,
            text_x,
            y + 10.0,
            mc::TEXT_WHITE,
            &title,
        );

        let meta = format!(
            "by {} • {} downloads{}",
            project.author,
            format_downloads(project.downloads),
            format_categories(project)
        );
        let meta = fitted(ui, Face::Regular, META_TEXT, text_w, &meta);
        mc_text_shadow(
            ui,
            Face::Regular,
            META_TEXT,
            text_x,
            y + 40.0,
            mc::TEXT_GOLD,
            &meta,
        );

        let about = starling_catalog::caution(project).unwrap_or_else(|| project.summary.clone());
        let mut line_y = y + 68.0;
        for line in fitted_lines(ui, Face::Regular, BODY_TEXT, text_w, 2, &about) {
            mc_text_shadow(
                ui,
                Face::Regular,
                BODY_TEXT,
                text_x,
                line_y,
                mc::TEXT_GRAY,
                &line,
            );
            line_y += BODY_LINE;
        }

        // 3. Right Action Buttons: Install / Remove, and "All Versions"
        let btn_x = MARGIN + row_w - ROW_BUTTON_W - 16.0;
        let install_rect = [btn_x, y + 14.0, ROW_BUTTON_W, ROW_BUTTON_H];
        let ver_rect = [
            btn_x,
            y + 14.0 + ROW_BUTTON_H + 8.0,
            ROW_BUTTON_W,
            VER_BUTTON_H,
        ];

        if shared.installing.as_deref() == Some(project.title.as_str()) {
            mc_button(ui, install_rect, "Installing...", false, false, None, drawn);
        } else if card.installed.contains(&project.id) {
            let (state, remove) = installed_parts(install_rect);
            mc_label_shadow(ui, state, "Installed", mc::TEXT_GREEN);
            mc_button(ui, remove, "Remove", false, true, None, drawn);
            if let Some(part) = list::shown_part(remove, top, bottom) {
                drawn.push((part, Act::Remove(i)));
            }
        } else {
            mc_button(ui, install_rect, "Install", false, true, None, drawn);
            if let Some(part) = list::shown_part(install_rect, top, bottom) {
                drawn.push((part, Act::Install(i)));
            }
        }

        mc_button(ui, ver_rect, "All Versions", false, true, None, drawn);
        if let Some(part) = list::shown_part(ver_rect, top, bottom) {
            drawn.push((part, Act::ViewVersions(i)));
        }
    }

    let y = top + content_h(rows.len()) - END_H - scrolled;
    let end = &shared.feed.tail();
    if matches!(end, Tail::Loading) {
        let line = loading_line(card.shelf, rows.is_empty());
        mc_text_shadow(
            ui,
            Face::Regular,
            BODY_TEXT,
            MARGIN + 14.0,
            y + 26.0,
            mc::TEXT_GRAY,
            &line,
        );
    } else if let Tail::End = end {
        let line = if rows.is_empty() {
            "Nothing found. Try another search."
        } else {
            "End of list."
        };
        mc_text_shadow(
            ui,
            Face::Regular,
            BODY_TEXT,
            MARGIN + 14.0,
            y + 26.0,
            mc::TEXT_GRAY,
            line,
        );
    } else if let Tail::Failed(why) = end {
        let text_w = row_w - ROW_BUTTON_W - 44.0;
        let line = fitted(ui, Face::Regular, BODY_TEXT, text_w, why);
        mc_text_shadow(
            ui,
            Face::Regular,
            BODY_TEXT,
            MARGIN + 14.0,
            y + 26.0,
            mc::TEXT_RED,
            &line,
        );
        let rect = [
            MARGIN + row_w - ROW_BUTTON_W - 16.0,
            y + (END_H - ROW_BUTTON_H) / 2.0,
            ROW_BUTTON_W,
            ROW_BUTTON_H,
        ];
        mc_button(ui, rect, "Try again", false, true, None, drawn);
        if let Some(part) = list::shown_part(rect, top, bottom) {
            drawn.push((part, Act::Retry));
        }
    }
}

/// The "All Versions" modal sheet for browsing, selecting and downloading versions.
fn draw_versions(
    ui: &mut Ui<'_>,
    card: &Catalog,
    shared: &Shared,
    drawn: &mut Vec<([f32; 4], Act)>,
) {
    let Some(project) = &card.versions_project else {
        return;
    };
    let (w, h) = (ui.w, ui.h);
    ui.fill(0.0, 0.0, w, h, mc::MODAL_DIM);

    let panel_w = 860.0_f32.min(w - 2.0 * MARGIN);
    let panel_h = 620.0_f32.min(h - 2.0 * MARGIN);
    let panel_x = (w - panel_w) / 2.0;
    let panel_y = (h - panel_h) / 2.0;

    // Beveled Minecraft modal frame
    ui.fill(panel_x, panel_y, panel_w, panel_h, mc::BTN_BORDER_BLACK);
    ui.fill(panel_x + 1.0, panel_y + 1.0, panel_w - 2.0, 1.0, mc::MODAL_BORDER_LIGHT);
    ui.fill(panel_x + 1.0, panel_y + 1.0, 1.0, panel_h - 2.0, mc::MODAL_BORDER_LIGHT);
    ui.fill(panel_x + panel_w - 2.0, panel_y + 1.0, 1.0, panel_h - 2.0, mc::MODAL_BORDER_DARK);
    ui.fill(panel_x + 1.0, panel_y + panel_h - 2.0, panel_w - 2.0, 1.0, mc::MODAL_BORDER_DARK);
    ui.fill(panel_x + 2.0, panel_y + 2.0, panel_w - 4.0, panel_h - 4.0, mc::MODAL_BG);

    // Header bar with separator
    let header_h = 80.0;
    ui.fill(panel_x + 2.0, panel_y + 2.0, panel_w - 4.0, header_h - 2.0, mc::HEADER_BG);
    ui.fill(panel_x + 2.0, panel_y + header_h - 1.0, panel_w - 4.0, 1.0, mc::SEPARATOR_DARK);
    ui.fill(panel_x + 2.0, panel_y + header_h, panel_w - 4.0, 1.0, mc::SEPARATOR_LIGHT);

    let title = format!("{} - All Versions", project.title);
    let title_fit = fitted(ui, Face::Semibold, TITLE_TEXT, panel_w - 380.0, &title);
    mc_text_shadow(
        ui,
        Face::Semibold,
        TITLE_TEXT,
        panel_x + 20.0,
        panel_y + 14.0,
        mc::TEXT_YELLOW,
        &title_fit,
    );

    let author_line = format!("by {} • {}", project.author, project.kind.label());
    mc_text_shadow(
        ui,
        Face::Regular,
        META_TEXT,
        panel_x + 20.0,
        panel_y + 46.0,
        mc::TEXT_GRAY,
        &author_line,
    );

    // Filter toggle button: "All MC Versions" vs "MC 26.3 Only"
    let filter_text = if card.versions_filter_current {
        "Version: 26.3 Only"
    } else {
        "Version: All Versions"
    };
    let filter_w = ui.measure(Face::Semibold, BUTTON_TEXT, filter_text) + 32.0;
    let filter_btn = [
        panel_x + panel_w - filter_w - 110.0,
        panel_y + 14.0,
        filter_w,
        52.0,
    ];
    mc_button(
        ui,
        filter_btn,
        filter_text,
        card.versions_filter_current,
        true,
        Some(Act::ToggleVersionsFilter),
        drawn,
    );

    // Close button
    let close_btn = [panel_x + panel_w - 94.0, panel_y + 14.0, 80.0, 52.0];
    mc_button(
        ui,
        close_btn,
        "Done",
        false,
        true,
        Some(Act::CloseVersions),
        drawn,
    );

    // Versions Content Area
    let list_top = panel_y + header_h + 10.0;
    let list_bottom = panel_y + panel_h - 10.0;
    let ver_row_h = 96.0;
    let ver_gap = 10.0;
    let ver_w = panel_w - 40.0;
    let ver_x = panel_x + 20.0;

    if card.versions_loading {
        let loading_txt = "Fetching versions from Modrinth...";
        let tw = ui.measure(Face::Semibold, 24.0, loading_txt);
        mc_text_shadow(
            ui,
            Face::Semibold,
            24.0,
            panel_x + (panel_w - tw) / 2.0,
            list_top + 100.0,
            mc::TEXT_GRAY,
            loading_txt,
        );
        return;
    }

    if let Some(err) = &card.versions_error {
        let err_fit = fitted(ui, Face::Regular, BODY_TEXT, panel_w - 40.0, err);
        mc_text_shadow(
            ui,
            Face::Regular,
            BODY_TEXT,
            panel_x + 20.0,
            list_top + 40.0,
            mc::TEXT_RED,
            &err_fit,
        );
        return;
    }

    let filtered_versions: Vec<(usize, &Version)> = card
        .versions_list
        .iter()
        .enumerate()
        .filter(|(_, v)| {
            if card.versions_filter_current {
                v.game_versions
                    .iter()
                    .any(|g| g == starling_catalog::browser::GAME)
            } else {
                true
            }
        })
        .collect();

    if filtered_versions.is_empty() {
        let empty_txt = "No versions found matching the filter.";
        let tw = ui.measure(Face::Regular, BODY_TEXT, empty_txt);
        mc_text_shadow(
            ui,
            Face::Regular,
            BODY_TEXT,
            panel_x + (panel_w - tw) / 2.0,
            list_top + 100.0,
            mc::TEXT_GRAY,
            empty_txt,
        );
        return;
    }

    let scrolled = card.versions_scroller.offset;
    for (vis_idx, (real_idx, version)) in filtered_versions.iter().enumerate() {
        let vy = list_top + vis_idx as f32 * ver_row_h - scrolled;
        let v_box = ver_row_h - ver_gap;
        if vy + v_box < list_top || vy > list_bottom {
            continue;
        }

        let ver_rect = [ver_x, vy, ver_w, v_box];
        let ver_hovered = mc_is_hovered(ui, ver_rect);
        let border_color = if ver_hovered {
            mc::SLOT_BORDER_HOVER
        } else {
            mc::SLOT_BORDER_IDLE
        };
        let bg_color = if ver_hovered {
            mc::SLOT_BG_HOVER
        } else {
            mc::SLOT_BG
        };

        // Minecraft slot box for version entry
        ui.fill(ver_x, vy, ver_w, v_box, mc::SLOT_BORDER_BLACK);
        ui.fill(ver_x + 1.0, vy + 1.0, ver_w - 2.0, 1.0, border_color);
        ui.fill(ver_x + 1.0, vy + 1.0, 1.0, v_box - 2.0, border_color);
        ui.fill(ver_x + ver_w - 2.0, vy + 1.0, 1.0, v_box - 2.0, border_color);
        ui.fill(ver_x + 1.0, vy + v_box - 2.0, ver_w - 2.0, 1.0, border_color);
        ui.fill(ver_x + 2.0, vy + 2.0, ver_w - 4.0, v_box - 4.0, bg_color);

        // Version title / number:
        let v_title = format!(
            "{} ({})",
            version.number,
            if version.loaders.is_empty() {
                "Pack"
            } else {
                &version.loaders[0]
            }
        );
        mc_text_shadow(
            ui,
            Face::Semibold,
            22.0,
            ver_x + 14.0,
            vy + 10.0,
            mc::TEXT_WHITE,
            &v_title,
        );

        // MC Versions supported & file size:
        let size_str = version.file().map_or("0 MB".into(), |f| {
            format!("{:.1} MB", f.size as f64 / (1024.0 * 1024.0))
        });
        let game_vers = if version.game_versions.len() > 5 {
            format!(
                "MC: {}, ... (+{})",
                version.game_versions[..4].join(", "),
                version.game_versions.len() - 4
            )
        } else {
            format!("MC: {}", version.game_versions.join(", "))
        };
        let date_str = if version.published.len() >= 10 {
            &version.published[..10]
        } else {
            &version.published
        };
        let sub = format!("{game_vers} • {size_str} • Published {date_str}");
        let sub_fit = fitted(ui, Face::Regular, 16.0, ver_w - 180.0, &sub);
        mc_text_shadow(
            ui,
            Face::Regular,
            16.0,
            ver_x + 14.0,
            vy + 42.0,
            mc::TEXT_GRAY,
            &sub_fit,
        );

        // Install button for this specific version:
        let v_btn_w = 150.0;
        let v_btn_h = 52.0;
        let v_btn = [
            ver_x + ver_w - v_btn_w - 14.0,
            vy + (v_box - v_btn_h) / 2.0,
            v_btn_w,
            v_btn_h,
        ];

        if shared.installing.as_deref() == Some(project.title.as_str()) {
            mc_button(ui, v_btn, "Installing...", false, false, None, drawn);
        } else {
            mc_button(ui, v_btn, "Download", false, true, None, drawn);
            if let Some(part) = list::shown_part(v_btn, list_top, list_bottom) {
                drawn.push((part, Act::InstallVersion(*real_idx)));
            }
        }
    }
}

/// The title, Close, the three shelves and the search box.
fn draw_head(ui: &mut Ui<'_>, card: &Catalog, edge: f32, drawn: &mut Vec<([f32; 4], Act)>) {
    mc_text_line_shadow(
        ui,
        Face::Semibold,
        TITLE_TEXT,
        62.0,
        MARGIN,
        8.0,
        mc::TEXT_WHITE,
        "Resource Pack & Shader Catalog",
    );
    mc_button(
        ui,
        [edge - 130.0, 12.0, 130.0, 52.0],
        "Done",
        false,
        true,
        Some(Act::Close),
        drawn,
    );

    const SHELF_Y: f32 = 78.0;
    const SHELF_H: f32 = 62.0;
    let mut x = MARGIN;
    for (shelf, name) in SHELVES {
        let width = ui.measure(Face::Semibold, BUTTON_TEXT, name) + 40.0;
        mc_button(
            ui,
            [x, SHELF_Y, width, SHELF_H],
            name,
            card.shelf == shelf,
            true,
            Some(Act::Shelf(shelf)),
            drawn,
        );
        x += width + 8.0;
    }

    // Classic Minecraft EditBox for search
    let search = [x + 4.0, SHELF_Y, (edge - x - 4.0).max(120.0), SHELF_H];
    let border_col = if card.typing {
        mc::EDITBOX_BORDER_FOCUSED
    } else {
        mc::EDITBOX_BORDER_IDLE
    };
    ui.fill(search[0], search[1], search[2], search[3], mc::BTN_BORDER_BLACK);
    ui.fill(search[0] + 1.0, search[1] + 1.0, search[2] - 2.0, search[3] - 2.0, border_col);
    ui.fill(search[0] + 2.0, search[1] + 2.0, search[2] - 4.0, search[3] - 4.0, mc::EDITBOX_BG);

    let (typed, ink) = if card.query.is_empty() && !card.typing {
        ("Search Modrinth & Registry...", mc::TEXT_DARK_GRAY)
    } else {
        (card.query.as_str(), mc::TEXT_WHITE)
    };
    let typed = fitted(
        ui,
        Face::Regular,
        BODY_TEXT,
        search[2] - 32.0 - CLEAR_W,
        typed,
    );
    mc_text_line_shadow(
        ui,
        Face::Regular,
        BODY_TEXT,
        search[3],
        search[0] + 16.0,
        search[1],
        ink,
        &typed,
    );
    if card.typing {
        let end = search[0] + 16.0 + ui.measure(Face::Regular, BODY_TEXT, &typed);
        ui.fill(end + 1.0, search[1] + 14.0, 2.0, search[3] - 28.0, mc::TEXT_WHITE);
    }
    if !card.query.is_empty() {
        let clear = [
            search[0] + search[2] - CLEAR_W,
            search[1] + 6.0,
            CLEAR_W - 6.0,
            search[3] - 12.0,
        ];
        mc_button(ui, clear, "Clear", false, true, Some(Act::Clear), drawn);
    }
    drawn.push((search, Act::Search));
}

/// The versions toggle on its shelf, and what an install says.
fn draw_foot(
    ui: &mut Ui<'_>,
    card: &Catalog,
    shared: &Shared,
    edge: f32,
    drawn: &mut Vec<([f32; 4], Act)>,
) {
    let foot = ui.h - FOOT_H + 10.0;
    let mut right = edge;
    if matches!(card.shelf, Shelf::ResourcePacks | Shelf::ShaderPacks) {
        let name = if card.everything {
            "Other game versions: On"
        } else {
            "Other game versions: Off"
        };
        let width = ui.measure(Face::Semibold, BUTTON_TEXT, name) + 40.0;
        right -= width;
        mc_button(
            ui,
            [right, foot, width, 56.0],
            name,
            card.everything,
            true,
            Some(Act::Everything),
            drawn,
        );
        right -= 12.0;
    }
    let status_w = (right - MARGIN).max(0.0);
    if shared.installing.is_some() {
        let line = fitted(ui, Face::Regular, BODY_TEXT, status_w, &shared.line);
        mc_text_shadow(ui, Face::Regular, BODY_TEXT, MARGIN, foot, mc::TEXT_WHITE, &line);
        // Minecraft progress bar: outer black, inner dark, green bar
        ui.fill(MARGIN, foot + 36.0, status_w, 14.0, mc::BTN_BORDER_BLACK);
        ui.fill(MARGIN + 1.0, foot + 37.0, status_w - 2.0, 12.0, mc::PROGRESS_BG);
        let done = (status_w * shared.fraction.clamp(0.0, 1.0)).max(6.0);
        ui.fill(MARGIN + 1.0, foot + 37.0, done - 2.0, 12.0, mc::PROGRESS_GREEN);
    } else if let Some((notice, failed)) = &card.notice {
        let ink = if *failed { mc::TEXT_RED } else { mc::TEXT_WHITE };
        let mut y = foot;
        for line in fitted_lines(ui, Face::Regular, BODY_TEXT, status_w, 2, notice) {
            mc_text_shadow(ui, Face::Regular, BODY_TEXT, MARGIN, y, ink, &line);
            y += BODY_LINE;
        }
    }
}

impl Game<'_> {
    /// A page's Catalog button: the screen, on that page's shelf.
    pub(super) fn open_catalog_on(&mut self, shelf: Shelf) {
        self.catalog.shelf = shelf;
        self.open_catalog();
    }

    /// The screen, on the shelf it was on.
    pub(super) fn open_catalog(&mut self) {
        starling_catalog::set_game_home(Self::game_dir());
        let card = &mut self.catalog;
        card.shown = true;
        card.notice = None;
        card.installed = starling_catalog::installed_projects(&Self::game_dir());
        self.catalog_search();
    }

    /// Back key: closes modal or screen.
    pub(super) fn catalog_back(&mut self) -> bool {
        let card = &mut self.catalog;
        if !card.shown {
            return false;
        }
        if card.versions_project.take().is_some() {
            return true;
        }
        if card.removing.take().is_none() {
            card.shown = false;
            card.typing = false;
        }
        true
    }

    fn refresh_installed(&mut self) {
        self.catalog.installed = starling_catalog::installed_projects(&Self::game_dir());
        self.catalog.checked_at = Some(Instant::now());
    }

    fn catalog_check_files(&mut self) {
        let card = &self.catalog;
        if card.shown && card.checked_at.is_none_or(|at| at.elapsed() >= FILE_CHECK) {
            self.refresh_installed();
        }
    }

    fn refresh_mods_page(&mut self, gone: Option<&str>) {
        let on = horizon_client::mod_list::ModList::load(&Self::game_dir()).enabled;
        let enabled = &mut self.settings.mods.enabled;
        enabled.extend(on);
        if let Some(id) = gone {
            enabled.remove(id);
        }
        self.start_mods();
    }

    fn catalog_ask_remove(&mut self, index: usize) {
        let project = self
            .catalog
            .shared
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .feed
            .rows()
            .get(index)
            .cloned();
        self.catalog.removing = project;
    }

    fn catalog_remove(&mut self, project: Project) {
        let busy = self
            .catalog
            .shared
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .installing
            .is_some();
        if busy {
            self.catalog.notice = Some(("Wait for the install to finish first.".to_owned(), true));
            return;
        }
        let title = project.title.clone();
        self.catalog.notice = Some(
            match starling_catalog::remove(&Self::game_dir(), &project) {
                Ok(entry) => {
                    self.after_removal(&entry);
                    (format!("{title} was removed."), false)
                }
                Err(e) => (remove_failed(&title, &e), true),
            },
        );
        self.refresh_installed();
    }

    fn after_removal(&mut self, entry: &Entry) {
        match entry.kind {
            Kind::Shader => self.after_shader_pack_removed(&entry.file),
            Kind::ResourcePack => {
                let infos =
                    super::packs::pack_infos(&self.vanilla_root(), &self.server_packs.active);
                self.ui.options.update_pack_infos(infos);
            }
            _ => {
                let id = starling_catalog::mod_id(entry);
                self.refresh_mods_page(id.as_deref());
            }
        }
    }

    fn after_shader_pack_removed(&mut self, file: &str) {
        let name = file.rsplit('/').next().unwrap_or(file);
        if self.settings.horizon.shader_pack == name {
            self.settings.horizon.shader_pack.clear();
            self.settings.horizon.shader_profile.clear();
            self.settings.save(&Self::options_path());
        }
        self.settings.shader_packs =
            horizon_client::shader_pack_list::scan(&Self::shader_packs_dir());
    }

    pub(super) fn catalog_typing(&self) -> bool {
        self.catalog.shown && self.catalog.typing
    }

    pub(super) fn catalog_query(&self) -> &str {
        &self.catalog.query
    }

    pub(super) fn catalog_type(&mut self, text: &str) {
        let card = &mut self.catalog;
        let room = QUERY_CHARS.saturating_sub(card.query.chars().count());
        card.query
            .extend(text.chars().filter(|c| !c.is_control()).take(room));
        card.typed_at = Some(Instant::now());
    }

    pub(super) fn catalog_backspace(&mut self) {
        let card = &mut self.catalog;
        card.query.pop();
        card.typed_at = Some(Instant::now());
    }

    pub(super) fn catalog_submit(&mut self) {
        let card = &mut self.catalog;
        card.typing = false;
        card.typed_at = None;
        self.catalog_search();
    }

    fn catalog_search(&mut self) {
        self.refresh_installed();
        self.catalog.list.reset();
        let ask = self
            .catalog
            .shared
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .feed
            .restart();
        self.catalog_ask(ask);
    }

    fn catalog_ask(&self, ask: Ask) {
        let card = &self.catalog;
        let shared = card.shared.clone();
        let worker = shared.clone();
        let (shelf, query, everything) = (card.shelf, card.query.clone(), card.everything);
        let started = std::thread::Builder::new()
            .name("catalog search".into())
            .spawn(move || {
                let found =
                    starling_catalog::search(shelf, &query, everything, ask.offset, list::PAGE)
                        .map_err(|e| search_failed(&e));
                worker
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .feed
                    .answer(ask, found);
            });
        if let Err(e) = started {
            shared
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .feed
                .answer(
                    ask,
                    Err(search_failed(&format!("The search didn't start: {e}"))),
                );
        }
    }

    fn catalog_list_size(&self) -> (f32, f32) {
        let [w, h] = self.touch_size();
        let view = h as f32 / density(w as f32, h as f32) - HEAD_H - FOOT_H;
        let rows = self
            .catalog
            .shared
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .feed
            .rows()
            .len();
        (view, content_h(rows))
    }

    fn catalog_keep_up(&mut self) {
        let (view, content) = self.catalog_list_size();
        let card = &mut self.catalog;
        card.list.clamp(content - view);
        if !list::near_end(card.list.offset, view, content, NEAR_END) {
            return;
        }
        let ask = card
            .shared
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .feed
            .next();
        if let Some(ask) = ask {
            self.catalog_ask(ask);
        }
    }

    pub(super) fn poll_catalog(&mut self) {
        if self.catalog.typing && self.keyboard_put_away("catalog") {
            self.catalog.typing = false;
        }
        if self
            .catalog
            .typed_at
            .is_some_and(|at| at.elapsed() >= TYPING_PAUSE)
        {
            self.catalog.typed_at = None;
            self.catalog.notice = None;
            self.catalog_search();
        }
        if self.catalog.shown {
            self.catalog_keep_up();

            // 1. Poll incoming background icons
            if let Some(rx) = &self.catalog.icon_rx {
                while let Ok((id, rgba)) = rx.try_recv() {
                    self.catalog.icons.insert(id, rgba);
                }
            }

            // 2. Poll incoming background versions
            if let Some(rx) = &self.catalog.versions_rx {
                while let Ok((id, res)) = rx.try_recv() {
                    if self
                        .catalog
                        .versions_project
                        .as_ref()
                        .is_some_and(|p| p.id == id)
                    {
                        self.catalog.versions_loading = false;
                        match res {
                            Ok(versions) => self.catalog.versions_list = versions,
                            Err(err) => self.catalog.versions_error = Some(err),
                        }
                    }
                }
            }

            // 3. Request icons for visible rows
            let (top, bottom) = (HEAD_H, {
                let [w, h] = self.touch_size();
                h as f32 / density(w as f32, h as f32) - FOOT_H
            });
            let (rows, scrolled) = {
                let s = self
                    .catalog
                    .shared
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner);
                (s.feed.rows().to_vec(), self.catalog.list.offset)
            };
            for i in list::rows_shown(scrolled, bottom - top, ROW_H, rows.len()) {
                let p = &rows[i];
                if !self.catalog.icons.contains_key(&p.id)
                    && !self.catalog.loading_icons.contains(&p.id)
                {
                    if let Some(url) = p.icon.clone() {
                        self.catalog.loading_icons.insert(p.id.clone());
                        let tx = self.catalog.icon_tx.clone();
                        let pid = p.id.clone();
                        let home = Self::game_dir();
                        std::thread::Builder::new()
                            .name("catalog icon".into())
                            .spawn(move || {
                                if let Ok(path) = starling_catalog::browser::images::fetch(
                                    &home,
                                    &url,
                                    starling_catalog::browser::images::ICON,
                                ) {
                                    if let Ok(bytes) = std::fs::read(&path) {
                                        if let Some(rgba) =
                                            horizon_client::pack_screen::load_icon(&bytes)
                                        {
                                            let _ = tx.send((pid, rgba));
                                        }
                                    }
                                }
                            })
                            .ok();
                    }
                }
            }
        }
        self.catalog_check_files();
        let finished = self
            .catalog
            .shared
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .finished
            .take();
        let Some((title, result)) = finished else {
            return;
        };
        self.catalog.notice = Some(match result {
            Ok(done) => {
                match done {
                    Done::NativeMod => self.list_new_native_mods(),
                    Done::ShaderPack => {
                        self.settings.shader_packs =
                            horizon_client::shader_pack_list::scan(&Self::shader_packs_dir());
                    }
                    Done::ResourcePack => {
                        let infos = super::packs::pack_infos(
                            &self.vanilla_root(),
                            &self.server_packs.active,
                        );
                        self.ui.options.update_pack_infos(infos);
                    }
                    Done::ContentMod => self.refresh_mods_page(None),
                }
                self.refresh_installed();
                (done_notice(done, &title), false)
            }
            Err(e) => (install_failed(&title, &e), true),
        });
    }

    fn catalog_install(&mut self, index: usize) {
        let card = &mut self.catalog;
        let shared = card.shared.clone();
        let worker = shared.clone();
        let project = {
            let mut s = shared.lock().unwrap_or_else(PoisonError::into_inner);
            if s.installing.is_some() {
                return;
            }
            let Some(project) = s.feed.rows().get(index).cloned() else {
                return;
            };
            s.installing = Some(project.title.clone());
            s.line = project.title.clone();
            s.fraction = 0.0;
            project
        };
        card.notice = None;
        let everything = card.everything;
        let home = Self::game_dir();
        let title = project.title.clone();
        let started = std::thread::Builder::new()
            .name("catalog install".into())
            .spawn(move || {
                let progress = |line: &str, fraction: f32| {
                    let mut s = worker.lock().unwrap_or_else(PoisonError::into_inner);
                    line.clone_into(&mut s.line);
                    s.fraction = fraction;
                };
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    starling_catalog::install(&project, everything, &home, &progress)
                }))
                .unwrap_or_else(|_| Err(format!("Installing {} stopped.", project.title)));
                let mut s = worker.lock().unwrap_or_else(PoisonError::into_inner);
                s.installing = None;
                s.finished = Some((project.title, result));
            });
        if let Err(e) = started {
            let mut s = shared.lock().unwrap_or_else(PoisonError::into_inner);
            s.installing = None;
            let why = format!("Installing {title} didn't start: {e}");
            s.finished = Some((title, Err(why)));
        }
    }

    fn catalog_open_versions(&mut self, index: usize) {
        let card = &mut self.catalog;
        let Some(project) = card
            .shared
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .feed
            .rows()
            .get(index)
            .cloned()
        else {
            return;
        };
        card.versions_project = Some(project.clone());
        card.versions_list.clear();
        card.versions_loading = true;
        card.versions_error = None;
        card.versions_scroller.reset();

        let tx = card.versions_tx.clone();
        let pid = project.id.clone();
        std::thread::Builder::new()
            .name("catalog versions".into())
            .spawn(move || {
                let res = starling_catalog::project_versions(&project);
                let _ = tx.send((pid, res));
            })
            .ok();
    }

    fn catalog_install_version(&mut self, version_idx: usize) {
        let card = &mut self.catalog;
        let Some(project) = card.versions_project.clone() else {
            return;
        };
        let Some(version) = card.versions_list.get(version_idx).cloned() else {
            return;
        };
        let shared = card.shared.clone();
        let worker = shared.clone();
        {
            let mut s = shared.lock().unwrap_or_else(PoisonError::into_inner);
            if s.installing.is_some() {
                return;
            }
            s.installing = Some(project.title.clone());
            s.line = format!("{} {}", project.title, version.number);
            s.fraction = 0.0;
        }
        card.notice = None;
        let home = Self::game_dir();
        let title = project.title.clone();
        std::thread::Builder::new()
            .name("catalog install version".into())
            .spawn(move || {
                let progress = |line: &str, fraction: f32| {
                    let mut s = worker.lock().unwrap_or_else(PoisonError::into_inner);
                    line.clone_into(&mut s.line);
                    s.fraction = fraction;
                };
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    starling_catalog::install_version(&project, &version, &home, &progress)
                }))
                .unwrap_or_else(|_| Err(format!("Installing {} stopped.", project.title)));
                let mut s = worker.lock().unwrap_or_else(PoisonError::into_inner);
                s.installing = None;
                s.finished = Some((project.title, result));
            })
            .ok();
    }

    /// The screen, over everything else while it is open.
    pub(super) fn draw_catalog(&self, batch: &mut HudBatch) {
        let card = &self.catalog;
        if !card.shown {
            return;
        }
        let shared = card.shared.lock().unwrap_or_else(PoisonError::into_inner);
        let mut ui = Ui::new(&self.gui.atlas, batch, self.cursor);
        let (w, h) = (ui.w, ui.h);
        let edge = w - MARGIN - if card.typing { w * NAV_BAR_SHARE } else { 0.0 };
        let mut drawn = Vec::new();
        // 1. Dark translucent in-world screen backdrop
        ui.fill(0.0, 0.0, w, h, mc::SCREEN_BG);

        // 2. Main catalog list
        draw_list(&mut ui, card, &shared, edge, &mut drawn);

        // 3. Header bar with Minecraft 2px separator
        ui.fill(0.0, 0.0, w, HEAD_H, mc::HEADER_BG);
        ui.fill(0.0, HEAD_H - 2.0, w, 1.0, mc::SEPARATOR_DARK);
        ui.fill(0.0, HEAD_H - 1.0, w, 1.0, mc::SEPARATOR_LIGHT);

        // 4. Footer bar with Minecraft 2px separator
        ui.fill(0.0, h - FOOT_H, w, FOOT_H, mc::HEADER_BG);
        ui.fill(0.0, h - FOOT_H, w, 1.0, mc::SEPARATOR_DARK);
        ui.fill(0.0, h - FOOT_H + 1.0, w, 1.0, mc::SEPARATOR_LIGHT);

        // 5. Header and Footer interactive contents
        draw_head(&mut ui, card, edge, &mut drawn);
        draw_foot(&mut ui, card, &shared, edge, &mut drawn);
        if let Some(project) = &card.removing {
            draw_confirm(&mut ui, &project.title, &mut drawn);
        }
        if card.versions_project.is_some() {
            draw_versions(&mut ui, card, &shared, &mut drawn);
        }
        ui.finish();
        *card.buttons.borrow_mut() = drawn;
    }

    fn catalog_act_at(&mut self, at: [f32; 2]) {
        let card = &mut self.catalog;
        let act = card
            .buttons
            .borrow()
            .iter()
            .find(|(r, _)| {
                at[0] >= r[0] && at[0] <= r[0] + r[2] && at[1] >= r[1] && at[1] <= r[1] + r[3]
            })
            .map(|(_, act)| act.clone());
        if !matches!(act, Some(Act::Search | Act::Clear)) {
            card.typing = false;
        }
        let Some(act) = act else {
            return;
        };
        self.catalog_handle_act(act);
    }

    fn catalog_handle_act(&mut self, act: Act) {
        let card = &mut self.catalog;
        match act {
            Act::Close => card.shown = false,
            Act::Shelf(shelf) => {
                card.shelf = shelf;
                card.typed_at = None;
                card.notice = None;
                self.catalog_search();
            }
            Act::Search => card.typing = true,
            Act::Clear => {
                card.query.clear();
                card.typed_at = None;
                self.catalog_search();
            }
            Act::Everything => {
                card.everything = !card.everything;
                self.catalog_search();
            }
            Act::Retry => {
                let ask = card
                    .shared
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .feed
                    .retry();
                if let Some(ask) = ask {
                    self.catalog_ask(ask);
                }
            }
            Act::Install(index) => self.catalog_install(index),
            Act::ViewVersions(index) => self.catalog_open_versions(index),
            Act::InstallVersion(version_idx) => self.catalog_install_version(version_idx),
            Act::ToggleVersionsFilter => {
                card.versions_filter_current = !card.versions_filter_current;
                card.versions_scroller.reset();
            }
            Act::CloseVersions => card.versions_project = None,
            Act::Remove(index) => self.catalog_ask_remove(index),
            Act::ConfirmRemove => {
                if let Some(project) = card.removing.take() {
                    self.catalog_remove(project);
                }
            }
            Act::CancelRemove => card.removing = None,
        }
    }

    /// While the screen is open it takes every touch.
    pub(super) fn catalog_touch(&mut self, touch: &Touch) -> bool {
        if !self.catalog.shown {
            return false;
        }
        let [w, h] = self.touch_size();
        let d = density(w as f32, h as f32);
        let at = [touch.location.x as f32 / d, touch.location.y as f32 / d];
        let bottom = h as f32 / d - FOOT_H;
        let (view, content) = self.catalog_list_size();
        let most = content - view;
        let card = &mut self.catalog;
        let tapped = match touch.phase {
            TouchPhase::Started => {
                let on_list = at[1] >= HEAD_H && at[1] < bottom;
                card.list.press(touch.id, at, on_list);
                None
            }
            TouchPhase::Moved => {
                card.list.moved(touch.id, at, most);
                None
            }
            TouchPhase::Ended => card.list.lift(touch.id, at, most),
            TouchPhase::Cancelled => {
                card.list.cancel(touch.id);
                None
            }
        };
        let Some(at) = tapped else {
            return true;
        };
        self.catalog_act_at(at);
        true
    }

    /// Mouse click handling on catalog screen: true if handled.
    pub(super) fn catalog_mouse_click(&mut self, pressed: bool, cursor: (f32, f32)) -> bool {
        if !self.catalog.shown {
            return false;
        }
        if !pressed {
            return true;
        }
        let [w, h] = self.touch_size();
        let d = density(w as f32, h as f32);
        let scale = self.renderer.as_ref().map_or(1.0, |r| {
            HudBatch::new([r.config.width, r.config.height]).scale
        });
        let at = [cursor.0 * scale / d, cursor.1 * scale / d];
        self.catalog_act_at(at);
        true
    }

    /// Mouse wheel scrolling on catalog screen: scrolls main list or versions list.
    pub(super) fn catalog_mouse_wheel(&mut self, dy: f32) -> bool {
        if !self.catalog.shown {
            return false;
        }
        if self.catalog.versions_project.is_some() {
            let ver_count = self.catalog.versions_list.len();
            let content_h = ver_count as f32 * 96.0;
            let [w, h] = self.touch_size();
            let view = h as f32 / density(w as f32, h as f32) - 100.0;
            let card = &mut self.catalog;
            card.versions_scroller.offset =
                (card.versions_scroller.offset - dy).clamp(0.0, (content_h - view).max(0.0));
        } else {
            let (view, content) = self.catalog_list_size();
            let most = (content - view).max(0.0);
            let card = &mut self.catalog;
            card.list.offset = (card.list.offset - dy).clamp(0.0, most);
            self.catalog_keep_up();
        }
        true
    }

    /// Keyboard input handling on catalog screen: typing in search or navigation.
    pub(super) fn catalog_key_event(&mut self, event: &KeyEvent) -> bool {
        if !self.catalog.shown || event.state != ElementState::Pressed {
            return false;
        }
        let card = &mut self.catalog;
        match &event.logical_key {
            Key::Named(NamedKey::Escape) => {
                if card.versions_project.is_some() {
                    card.versions_project = None;
                } else if card.typing {
                    card.typing = false;
                } else {
                    card.shown = false;
                }
                return true;
            }
            Key::Named(NamedKey::Backspace) => {
                if card.typing {
                    self.catalog_backspace();
                    return true;
                }
            }
            Key::Named(NamedKey::Enter) => {
                if card.typing {
                    self.catalog_submit();
                    return true;
                }
            }
            Key::Named(NamedKey::ArrowUp) => {
                self.catalog_mouse_wheel(60.0);
                return true;
            }
            Key::Named(NamedKey::ArrowDown) => {
                self.catalog_mouse_wheel(-60.0);
                return true;
            }
            Key::Character(s) => {
                if card.typing {
                    self.catalog_type(s.as_str());
                    return true;
                }
            }
            _ => {}
        }
        false
    }
}
