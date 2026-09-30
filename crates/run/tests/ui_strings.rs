//! The HUD's pixel font (m6x11plus) has ASCII and `×` but no `·`, `—`, `–` or `…`.
//! Every string the HUD shows from the rules crates must stay within that set.

use tc_core::SpellId;
use tc_core::rules::{CastBlock, TargetBlock};
use tc_run::{LEVELS, catalog};

fn check(bad: &mut Vec<String>, s: &str) {
    if let Some(c) = s.chars().find(|&c| !(c == ' ' || c.is_ascii_graphic() || c == '×')) {
        bad.push(format!("{c:?} in {s:?}"));
    }
}

#[test]
fn ui_strings_fit_pixel_font() {
    let mut bad = Vec::new();
    for spell in SpellId::ALL {
        check(&mut bad, spell.effect_text());
        check(&mut bad, spell.no_target_hint());
        check(&mut bad, spell.target_hint());
        check(&mut bad, CastBlock::NoTargets(spell).message());
        check(&mut bad, TargetBlock::NotATarget(spell).message());
    }
    for b in [
        CastBlock::GameOver,
        CastBlock::CardUsed,
        CastBlock::InCheck,
        CastBlock::ExposesKing,
        CastBlock::TooEarly,
    ] {
        check(&mut bad, b.message());
    }
    for b in [TargetBlock::ExposesKing, TargetBlock::KingSquare, TargetBlock::Obstacle, TargetBlock::Void] {
        check(&mut bad, b.message());
    }
    for item in catalog() {
        check(&mut bad, &item.name);
        check(&mut bad, &item.description);
    }
    for level in LEVELS.iter() {
        check(&mut bad, level.name);
    }
    assert!(bad.is_empty(), "strings the pixel font can't draw:\n{}", bad.join("\n"));
}
