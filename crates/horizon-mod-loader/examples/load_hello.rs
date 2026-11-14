//! Loads a library mod from the command line, the way the client will: the whole Phase A chain
//! without a phone.
//!
//! ```text
//! cargo build -p horizon-mod-hello
//! cargo run -p horizon-mod-loader --example load_hello -- target/debug
//! ```
//!
//! The argument is the folder the client would read (`horizon-native-mods`), so the example is the
//! client's four hooks and nothing else: list, start, frame twice, ask for the rows, tap one.

use horizon_mod_loader::{Host, frame, list, scan, settings, settings_click, start};

/// What the example offers a mod in place of a game.
struct NoGame {
    settings: std::cell::RefCell<std::collections::HashMap<(String, String), String>>,
}

impl Host for NoGame {
    fn log(&self, level: u32, text: &str) {
        println!("  [mod log {level}] {text}");
    }

    fn setting(&self, id: &str, key: &str) -> Option<String> {
        self.settings
            .borrow()
            .get(&(id.to_owned(), key.to_owned()))
            .cloned()
    }

    fn set_setting(&self, id: &str, key: &str, value: &str) {
        self.settings
            .borrow_mut()
            .insert((id.to_owned(), key.to_owned()), value.to_owned());
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let folder = std::env::args()
        .nth(1)
        .ok_or("usage: load_hello <folder of mod libraries>")?;
    let host = NoGame {
        settings: Default::default(),
    };

    scan(&[folder.clone().into()], &host);
    let found = list();
    if found.is_empty() {
        return Err(format!("no mod library in {folder}").into());
    }
    for info in &found {
        println!("found {} ({})", info.id, info.title);
        println!("  {}", info.description);
    }

    start(&host);
    frame(&host, 0.0);
    frame(&host, 0.05);
    println!("two frames ran");

    for info in &found {
        let mut rows = Vec::new();
        settings(&host, &info.id, &mut rows);
        println!("{} rows for {}:", rows.len(), info.id);
        for (n, row) in rows.iter().enumerate() {
            println!("  {n}. {row}");
        }
        settings_click(&host, &info.id, 1);
        println!("tapped row 1 of {}", info.id);
    }
    Ok(())
}
