// Copyright (C) 2026 Baraka Malila
// GPL-3.0-or-later
use std::cell::RefCell;

thread_local! {
    static ACCENT_PROVIDER: RefCell<Option<gtk4::CssProvider>> = const { RefCell::new(None) };
}

pub fn accent_for_product(product_name: &str) -> &'static str {
    let name = product_name.to_lowercase();
    if name.contains("proart") {
        "#00bcd4"
    } else if name.contains("rog") {
        "#e8002d"
    } else {
        "#e8a800" // TUF + unknown → Amber
    }
}

fn build_accent_css(hex: &str) -> String {
    let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(232);
    let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(168);
    let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(0);
    let g2 = (g as i16 - 30).max(0) as u8;
    let b2 = (b as u16 + 20).min(255) as u8;
    format!(
        "* {{ --utu-accent: {hex}; --utu-accent-end: #{r:02x}{g2:02x}{b2:02x}; \
         --utu-accent-glow: rgba({r},{g},{b},0.05); \
         --utu-accent-dim: rgba({r},{g},{b},0.15); }}"
    )
}

pub fn apply_accent(hex: &str) {
    let css = build_accent_css(hex);
    ACCENT_PROVIDER.with(|cell| {
        let mut opt = cell.borrow_mut();
        let provider = opt.get_or_insert_with(|| {
            let p = gtk4::CssProvider::new();
            if let Some(display) = gtk4::gdk::Display::default() {
                gtk4::style_context_add_provider_for_display(
                    &display,
                    &p,
                    gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
                );
            }
            p
        });
        provider.load_from_string(&css);
    });
}

pub fn detect_and_apply() {
    let hex = std::fs::read_to_string("/sys/class/dmi/id/product_name")
        .map(|s| accent_for_product(s.trim()))
        .unwrap_or("#e8a800");
    apply_accent(hex);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tuf_maps_to_amber() {
        assert_eq!(accent_for_product("ASUS TUF Gaming A15"), "#e8a800");
    }
    #[test]
    fn proart_maps_to_teal() {
        assert_eq!(accent_for_product("ASUS ProArt Studiobook"), "#00bcd4");
    }
    #[test]
    fn rog_maps_to_crimson() {
        assert_eq!(accent_for_product("ROG Zephyrus G14"), "#e8002d");
    }
    #[test]
    fn unknown_maps_to_amber() {
        assert_eq!(accent_for_product("Unknown Laptop"), "#e8a800");
    }
}
