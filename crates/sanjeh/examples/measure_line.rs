//! The README's example: a line measured, a word's sound, a line's melts and
//! a word's level.
//!
//!     cargo run -q --example measure_line

use sanjeh::line::{LineInput, default_model, measure};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let line: LineInput = serde_json::from_str(r#"{"text": "Where did you put the keys?", "duration_ms": 1400}"#)?;
    let measured = measure(&line, default_model());
    println!("{:.1} to hear, {:.1} to say", measured.ear.level, measured.mouth.level);

    println!("{:?}", sanjeh::pronounce("thought", sanjeh::Accent::Gb));
    for melt in sanjeh::connected::melts("pick it up", sanjeh::Accent::Us) {
        println!("{}: {}", melt.rule, melt.sound);
    }
    println!("{:?}", sanjeh::levels::word_level("grieving"));
    Ok(())
}
