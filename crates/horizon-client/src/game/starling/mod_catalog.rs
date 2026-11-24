// game-module-cfg: target_os = "android"
//! The in-game Resource Pack, Shader Pack, and Mod downloader / catalog.
//! Allows players to search, view icons, read descriptions, browse all available
//! versions, and install directly into `resourcepacks/` or `shaderpacks/` from
//! within Minecraft without leaving the game or using an external launcher.use super::Game;
use super::native_builds::{density, wrapped};
use horizon_client::{
    font::Patch,
    hud::{HudBatch, PACK_ICON_SIZE, PACK_ICON_SLOTS},
    ui_theme::{Face, Ui, color},
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

/// `text` in the middle of `rect`.
fn label(ui: &mut Ui<'_>, rect: [f32; 4], ink: u32, text: &str) {
    let width = ui.measure(Face::Semibold, BUTTON_TEXT, text);
    ui.text_line(
        Face::Semibold,
        BUTTON_TEXT,
        rect[3],
        rect[0] + (rect[2] - width) / 2.0,
        rect[1],
        ink,
        text,
    );
}

/// A button's look: filled with the accent when `lit`.
fn plate(ui: &mut Ui<'_>, rect: [f32; 4], text: &str, lit: bool) {
    let (fill, ink) = if lit {
        (color::ACCENT, color::ON_ACCENT)
    } else {
        (color::raised(), color::text())
    };
    ui.rounded(rect[0], rect[1], rect[2], rect[3], 10.0, fill);
    label(ui, rect, ink, text);
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
    plate(ui, rect, text, lit);
    drawn.push((rect, act));
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
    ui.fill(0.0, 0.0, w, h, color::hex(0x00_00_00, 190));
    let width = CONFIRM_W.min(w - 2.0 * MARGIN);
    let panel = [(w - width) / 2.0, (h - CONFIRM_H) / 2.0, width, CONFIRM_H];
    ui.rounded(panel[0], panel[1], panel[2], panel[3], 14.0, color::bg());
    ui.rounded(
        panel[0],
        panel[1],
        panel[2],
        panel[3],
        14.0,
        color::raised(),
    );
    let inner = width - 48.0;
    let head = fitted(
        ui,
        Face::Semibold,
        TITLE_TEXT,
        inner,
        &format!("Remove {title}?"),
    );
    ui.text(
        Face::Semibold,
        TITLE_TEXT,
        panel[0] + 24.0,
        panel[1] + 20.0,
        color::text(),
        &head,
    );
    let body = "This deletes its file from your game folder. You can install it again from here.";
    let mut y = panel[1] + 70.0;
    for line in fitted_lines(ui, Face::Regular, BODY_TEXT, inner, 3, body) {
        ui.text(
            Face::Regular,
            BODY_TEXT,
            panel[0] + 24.0,
            y,
            color::MUTED,
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
    ui.rounded(
        remove[0],
        remove[1],
        remove[2],
        remove[3],
        10.0,
        color::DANGER,
    );
    label(ui, remove, color::text(), "Remove");
    drawn.push((remove, Act::ConfirmRemove));
    plate(ui, cancel, "Cancel", false);
    drawn.push((cancel, Act::CancelRemove));
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
        ui.rounded(MARGIN, y, row_w, row, 12.0, color::raised());

        // 1. Pack Preview Icon
        let icon_x = MARGIN + 12.0;
        let icon_y = y + (row - ICON_SIZE) / 2.0;
        ui.rounded(icon_x, icon_y, ICON_SIZE, ICON_SIZE, 10.0, color::hover());

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
                ui.blit_sprite(icon_x, icon_y, ICON_SIZE, ICON_SIZE, rect);
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
        ui.text(
            Face::Semibold,
            TITLE_TEXT,
            text_x,
            y + 10.0,
            color::text(),
            &title,
        );

        let meta = format!(
            "by {} • {} downloads{}",
            project.author,
            format_downloads(project.downloads),
            format_categories(project)
        );
        let meta = fitted(ui, Face::Regular, META_TEXT, text_w, &meta);
        ui.text(
            Face::Regular,
            META_TEXT,
            text_x,
            y + 40.0,
            color::hex(0x56c476, 255),
            &meta,
        );

        let about = starling_catalog::caution(project).unwrap_or_else(|| project.summary.clone());
        let mut line_y = y + 68.0;
        for line in fitted_lines(ui, Face::Regular, BODY_TEXT, text_w, 2, &about) {
            ui.text(
                Face::Regular,
                BODY_TEXT,
                text_x,
                line_y,
                color::MUTED,
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
            label(ui, install_rect, color::MUTED, "Installing...");
        } else if card.installed.contains(&project.id) {
            let (state, remove) = installed_parts(install_rect);
            label(ui, state, color::MUTED, "Installed");
            ui.rounded(
                remove[0],
                remove[1],
                remove[2],
                remove[3],
                10.0,
                color::hover(),
            );
            label(ui, remove, color::text(), "Remove");
            if let Some(part) = list::shown_part(remove, top, bottom) {
                drawn.push((part, Act::Remove(i)));
            }
        } else {
            plate(ui, install_rect, "Install", true);
            if let Some(part) = list::shown_part(install_rect, top, bottom) {
                drawn.push((part, Act::Install(i)));
            }
        }

        plate(ui, ver_rect, "All Versions", false);
        if let Some(part) = list::shown_part(ver_rect, top, bottom) {
            drawn.push((part, Act::ViewVersions(i)));
        }
    }

    let y = top + content_h(rows.len()) - END_H - scrolled;
    if y >= bottom {
        return;
    }
    let (line, ink) = match shared.feed.tail() {
        Tail::Loading => (loading_line(card.shelf, rows.is_empty()), color::MUTED),
        Tail::More => (loading_line(card.shelf, false), color::MUTED),
        Tail::End if rows.is_empty() => ("Nothing found.".to_owned(), color::MUTED),
        Tail::End => ("That's all of them.".to_owned(), color::MUTED),
        Tail::Failed(why) => (why.clone(), color::text()),
    };
    let lines = fitted_lines(ui, Face::Regular, BODY_TEXT, text_w, 2, &line);
    let mut line_y = y + (END_H - lines.len() as f32 * BODY_LINE) / 2.0;
    for line in lines {
        ui.text(Face::Regular, BODY_TEXT, MARGIN + 14.0, line_y, ink, &line);
        line_y += BODY_LINE;
    }
    if matches!(shared.feed.tail(), Tail::Failed(_)) {
        let rect = [
            MARGIN + row_w - ROW_BUTTON_W - 16.0,
            y + (END_H - ROW_BUTTON_H) / 2.0,
            ROW_BUTTON_W,
            ROW_BUTTON_H,
        ];
        plate(ui, rect, "Try again", false);
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
    ui.fill(0.0, 0.0, w, h, color::hex(0x00_00_00, 215));

    let panel_w = 860.0_f32.min(w - 2.0 * MARGIN);
    let panel_h = 620.0_f32.min(h - 2.0 * MARGIN);
    let panel_x = (w - panel_w) / 2.0;
    let panel_y = (h - panel_h) / 2.0;

    ui.rounded(panel_x, panel_y, panel_w, panel_h, 14.0, color::BG);
    ui.border(panel_x, panel_y, panel_w, panel_h, 14.0, color::BORDER);

    // Header:
    let header_h = 80.0;
    ui.rounded(panel_x, panel_y, panel_w, header_h, 14.0, color::RAISED);

    let title = format!("{} - All Versions", project.title);
    let title_fit = fitted(ui, Face::Semibold, TITLE_TEXT, panel_w - 380.0, &title);
    ui.text(
        Face::Semibold,
        TITLE_TEXT,
        panel_x + 20.0,
        panel_y + 14.0,
        color::TEXT,
        &title_fit,
    );

    let author_line = format!("by {} • {}", project.author, project.kind.label());
    ui.text(
        Face::Regular,
        META_TEXT,
        panel_x + 20.0,
        panel_y + 46.0,
        color::MUTED,
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
    plate(ui, filter_btn, filter_text, card.versions_filter_current);
    drawn.push((filter_btn, Act::ToggleVersionsFilter));

    // Close button
    let close_btn = [panel_x + panel_w - 94.0, panel_y + 14.0, 80.0, 52.0];
    plate(ui, close_btn, "Close", false);
    drawn.push((close_btn, Act::CloseVersions));

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
        ui.text(
            Face::Semibold,
            24.0,
            panel_x + (panel_w - tw) / 2.0,
            list_top + 100.0,
            color::MUTED,
            loading_txt,
        );
        return;
    }

    if let Some(err) = &card.versions_error {
        let err_fit = fitted(ui, Face::Regular, BODY_TEXT, panel_w - 40.0, err);
        ui.text(
            Face::Regular,
            BODY_TEXT,
            panel_x + 20.0,
            list_top + 40.0,
            color::DANGER,
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
        ui.text(
            Face::Regular,
            BODY_TEXT,
            panel_x + (panel_w - tw) / 2.0,
            list_top + 100.0,
            color::MUTED,
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

        ui.rounded(ver_x, vy, ver_w, v_box, 10.0, color::RAISED);

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
        ui.text(
            Face::Semibold,
            22.0,
            ver_x + 14.0,
            vy + 10.0,
            color::TEXT,
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
        ui.text(
            Face::Regular,
            16.0,
            ver_x + 14.0,
            vy + 42.0,
            color::MUTED,
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
            label(ui, v_btn, color::MUTED, "Installing...");
        } else {
            plate(ui, v_btn, "Download", true);
            if let Some(part) = list::shown_part(v_btn, list_top, list_bottom) {
                drawn.push((part, Act::InstallVersion(*real_idx)));
            }
        }
    }
}

/// The title, Close, the three shelves and the search box.
fn draw_head(ui: &mut Ui<'_>, card: &Catalog, edge: f32, drawn: &mut Vec<([f32; 4], Act)>) {
    ui.text_line(
        Face::Semibold,
        TITLE_TEXT,
        62.0,
        MARGIN,
        8.0,
        color::text(),
        "Catalog",
    );
    button(
        ui,
        [edge - 130.0, 8.0, 130.0, 62.0],
        "Close",
        false,
        Act::Close,
        drawn,
    );

    const SHELF_Y: f32 = 78.0;
    const SHELF_H: f32 = 72.0;
    let mut x = MARGIN;
    for (shelf, name) in SHELVES {
        let width = ui.measure(Face::Semibold, BUTTON_TEXT, name) + 44.0;
        button(
            ui,
            [x, SHELF_Y, width, SHELF_H],
            name,
            card.shelf == shelf,
            Act::Shelf(shelf),
            drawn,
        );
        x += width + 8.0;
    }
    let search = [x + 4.0, SHELF_Y, (edge - x - 4.0).max(120.0), SHELF_H];
    ui.rounded(
        search[0],
        search[1],
        search[2],
        search[3],
        10.0,
        color::raised(),
    );
    if card.typing {
        ui.border(
            search[0],
            search[1],
            search[2],
            search[3],
            10.0,
            color::ACCENT,
        );
    }
    let (typed, ink) = if card.query.is_empty() && !card.typing {
        ("Search Modrinth & Registry...", color::MUTED)
    } else {
        (card.query.as_str(), color::text())
    };
    let typed = fitted(
        ui,
        Face::Regular,
        BODY_TEXT,
        search[2] - 32.0 - CLEAR_W,
        typed,
    );
    ui.text_line(
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
        ui.fill(end + 1.0, search[1] + 18.0, 2.0, 36.0, color::ACCENT);
    }
    if !card.query.is_empty() {
        let clear = [
            search[0] + search[2] - CLEAR_W,
            search[1],
            CLEAR_W,
            search[3],
        ];
        label(ui, clear, color::MUTED, "Clear");
        drawn.push((clear, Act::Clear));
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
    let foot = ui.h - FOOT_H + 7.0;
    let mut right = edge;
    if matches!(card.shelf, Shelf::ResourcePacks | Shelf::ShaderPacks) {
        let name = if card.everything {
            "Other game versions: On"
        } else {
            "Other game versions: Off"
        };
        let width = ui.measure(Face::Semibold, BUTTON_TEXT, name) + 44.0;
        right -= width;
        button(
            ui,
            [right, foot, width, 64.0],
            name,
            card.everything,
            Act::Everything,
            drawn,
        );
        right -= 12.0;
    }
    let status_w = (right - MARGIN).max(0.0);
    if shared.installing.is_some() {
        let line = fitted(ui, Face::Regular, BODY_TEXT, status_w, &shared.line);
        ui.text(Face::Regular, BODY_TEXT, MARGIN, foot, color::text(), &line);
        ui.rounded(MARGIN, foot + 38.0, status_w, 12.0, 6.0, color::raised());
        let done = (status_w * shared.fraction.clamp(0.0, 1.0)).max(12.0);
        ui.rounded(MARGIN, foot + 38.0, done, 12.0, 6.0, color::ACCENT);
    } else if let Some((notice, failed)) = &card.notice {
        let ink = if *failed {
            color::DANGER
        } else {
            color::text()
        };
        let mut y = foot;
        for line in fitted_lines(ui, Face::Regular, BODY_TEXT, status_w, 2, notice) {
            ui.text(Face::Regular, BODY_TEXT, MARGIN, y, ink, &line);
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
                let infos = self.resource_pack_infos();
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
                // The native mods Compile v2 can't play on this build are left off the shelf.
                if shelf == Shelf::Mods {
                    starling_catalog::hide_native_mods(
                        horizon_client::starling_store::catalog_hidden()
                            .iter()
                            .cloned(),
                    );
                }
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
                    let icon_src = p.icon.clone().or_else(|| {
                        let candidate1 = format!("native-mods/{}/icon.png", p.id);
                        let candidate2 = format!("horizon-native-mods/{}/icon.png", p.id);
                        if std::path::Path::new(&candidate1).is_file() {
                            Some(format!("file://{candidate1}"))
                        } else if std::path::Path::new(&candidate2).is_file() {
                            Some(format!("file://{candidate2}"))
                        } else {
                            None
                        }
                    });
                    if let Some(url) = icon_src {
                        self.catalog.loading_icons.insert(p.id.clone());
                        let tx = self.catalog.icon_tx.clone();
                        let pid = p.id.clone();
                        let home = Self::game_dir();
                        std::thread::Builder::new()
                            .name("catalog icon".into())
                            .spawn(move || {
                                let bytes_opt = if let Some(local_path) = url.strip_prefix("file://") {
                                    std::fs::read(local_path).ok()
                                } else if let Ok(path) = starling_catalog::browser::images::fetch(
                                    &home,
                                    &url,
                                    starling_catalog::browser::images::ICON,
                                ) {
                                    std::fs::read(&path).ok()
                                } else {
                                    None
                                };
                                if let Some(bytes) = bytes_opt {
                                    if let Some(rgba) =
                                        horizon_client::pack_screen::load_icon(&bytes)
                                    {
                                        let _ = tx.send((pid, rgba));
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
                        let infos = self.resource_pack_infos();
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
        ui.fill(0.0, 0.0, w, h, color::bg());
        draw_list(&mut ui, card, &shared, edge, &mut drawn);
        // The rows go under these two.
        ui.fill(0.0, 0.0, w, HEAD_H, color::bg());
        ui.fill(0.0, h - FOOT_H, w, FOOT_H, color::bg());
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
