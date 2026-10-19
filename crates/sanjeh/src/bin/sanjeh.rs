//! `sanjeh`, the measures from the command line.
//!
//! ```text
//! sanjeh pronounce [--gb] <word or phrase>             its sound in IPA, American or British
//! sanjeh melts [--gb] <line>                           where its words melt, and the line as said
//! sanjeh level <word>…                                 each word's CEFR level, by the open lists
//! sanjeh alike [--us|--gb] [--level <A1…C2>] [--language fa] [--kind same,ending,near] [--most <n>] <word>
//!                                                      the known words that sound like it
//! sanjeh line show [--gb] --ms <duration> <line>      a line measured, in plain words
//! sanjeh line features <items.jsonl>                  each item's features, for fitting a model
//! sanjeh line measure  <lines.jsonl> [--model <file>]  each line measured
//! sanjeh line bench    <items.jsonl> [--model <file>] [--split fit|test|all]
//! ```
//!
//! An items file is a benchmark (`sanjeh::bench`); `measure` also takes
//! bare line inputs, one a line. `-` reads standard input. Without
//! `--model`, the default model (`sanjeh::line::default_model`).

use std::process::ExitCode;

use sanjeh::alike::{AlikeKind, AlikeOptions, alikes, languages};
use sanjeh::bench::{Item, report};
use sanjeh::connected::{melted, melts};
use sanjeh::levels::Level;
use sanjeh::levels::word_level;
use sanjeh::line::{LineInput, LineModel, Reason, Skill, SkillMeasure, WhyNot, facts, measure};
use sanjeh::{Accent, Pronounced, pronounce, readings};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(problem) => {
            eprintln!("sanjeh: {problem}");
            ExitCode::FAILURE
        }
    }
}

fn option<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|at| args.get(at + 1))
        .map(String::as_str)
}

/// A file's text, or standard input's for `-`.
fn read_text(path: &str) -> Result<String, String> {
    if path == "-" {
        let mut text = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut text).map_err(|e| format!("standard input: {e}"))?;
        return Ok(text);
    }
    std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))
}

fn read_items(path: &str) -> Result<Vec<Item>, String> {
    let text = read_text(path)?;
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .enumerate()
        .map(|(at, l)| serde_json::from_str(l).map_err(|e| format!("{path}:{}: {e}", at + 1)))
        .collect()
}

/// The model `--model` names, or the default.
fn read_model(args: &[String]) -> Result<LineModel, String> {
    let Some(path) = option(args, "--model") else {
        return Ok(sanjeh::line::default_model().clone());
    };
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    LineModel::from_json(&text).map_err(|e| format!("{path}: {e}"))
}

/// The words after a command, without its flags and their values, as one text.
fn text_of(args: &[String]) -> String {
    let mut words = vec![];
    let mut rest = args.iter().skip(1);
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--ms" | "--model" | "--level" | "--kind" | "--most" | "--language" => {
                rest.next();
            }
            flag if flag.starts_with("--") => {}
            word => words.push(word),
        }
    }
    words.join(" ")
}

/// A reason in plain words.
fn said(reason: &Reason) -> String {
    match reason {
        Reason::Pace { words, seconds, .. } => format!("{words} words in {seconds} s"),
        Reason::Written { word, full } => format!("{word} = {full}"),
        Reason::Melt { rule, text, sound } => format!("{text} → {sound} ({rule})"),
        Reason::WeakEar { word } => format!("a recogniser stumbled on {word}"),
        Reason::HardWord { word, level } => format!("{word} ({level:?})"),
        Reason::HardSound { sound, word } => format!("{sound} in {word}"),
        Reason::Teaches { term, .. } => format!("teaches {term}"),
        Reason::Everyday { share } => format!("{:.0}% everyday words", share * 100.0),
        Reason::Tune { question } => (if *question { "a question" } else { "said with feeling" }).to_string(),
    }
}

/// Why a line can't be practised, in plain words.
fn why(why: &WhyNot) -> String {
    match why {
        WhyNot::NotASentence => "not a whole sentence".into(),
        WhyNot::TwoVoices => "two voices".into(),
        WhyNot::TooShort { words } => format!("too short ({words} words)"),
        WhyNot::TooLong { words } => format!("too long ({words} words)"),
        WhyNot::Spoken { ms } => format!("said in {:.1} s, too fast or slow to take whole", *ms as f64 / 1000.0),
        WhyNot::Names { words } => format!("mostly names ({})", words.join(", ")),
        WhyNot::SaidWrong { words } => format!("said wrong on purpose ({})", words.join(", ")),
        WhyNot::NotClear => "not clear enough to hear".into(),
    }
}

fn show(skill: &str, measure: &SkillMeasure) {
    let can = if measure.usable {
        "can be practised".to_string()
    } else {
        format!(
            "can't be practised: {}",
            measure.why_not.iter().map(why).collect::<Vec<_>>().join(", ")
        )
    };
    println!(
        "{skill:<6}{:?} ({:.1}) · worth {:.2} · {can}",
        Level::of_rank(measure.level),
        measure.level,
        measure.worth
    );
    for reason in &measure.reasons {
        println!("      {}", said(reason));
    }
}

fn accent_of(args: &[String]) -> Accent {
    if args.iter().any(|a| a == "--gb") {
        Accent::Gb
    } else {
        Accent::Us
    }
}

fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("pronounce") => {
            let text = text_of(args);
            match pronounce(&text, accent_of(args)) {
                Pronounced::Known(sound) => println!("{sound}"),
                Pronounced::Ambiguous => println!(
                    "read more than one way: {}",
                    readings(&text, accent_of(args)).join(", ")
                ),
                Pronounced::Unknown => println!("not in the table"),
            }
            return Ok(());
        }
        Some("melts") => {
            let text = text_of(args);
            let found = melts(&text, accent_of(args));
            // The line with each melted stretch as it is said; spans count UTF-16 units.
            let units: Vec<u16> = text.encode_utf16().collect();
            let mut said = String::new();
            let mut at = 0;
            for piece in melted(&text, &found) {
                said += &String::from_utf16_lossy(&units[at..piece.span.start]);
                said += &piece.said;
                at = piece.span.end;
            }
            said += &String::from_utf16_lossy(&units[at..]);
            println!("{said}");
            for melt in found {
                println!("  {:<13} {}", melt.rule, melt.sound);
            }
            return Ok(());
        }
        Some("line") if args.get(1).map(String::as_str) == Some("show") => {
            let ms: u32 = option(args, "--ms")
                .and_then(|ms| ms.parse().ok())
                .ok_or("--ms <how long it is said, in milliseconds> is needed")?;
            let input = LineInput {
                text: text_of(&args[1..]),
                duration_ms: ms,
                accent: accent_of(args),
                voices: 1,
                heard: None,
                entries: vec![],
                names: vec![],
            };
            let measured = measure(&input, &read_model(args)?);
            show("ear", &measured.ear);
            show("mouth", &measured.mouth);
            return Ok(());
        }
        Some("alike") => {
            let word = text_of(args);
            let level = match option(args, "--level") {
                Some(level) => Some(Level::parse(level).ok_or(format!("--level {level}: one of A1 to C2"))?),
                None => None,
            };
            let kinds = option(args, "--kind")
                .map(|kinds| {
                    kinds
                        .split(',')
                        .map(|kind| AlikeKind::parse(kind).ok_or(format!("--kind {kind}: same, ending or near")))
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()?
                .unwrap_or_default();
            let most = match option(args, "--most") {
                Some(most) => most.parse().map_err(|_| format!("--most {most}: a number"))?,
                None => 10,
            };
            let accent = if args.iter().any(|a| a == "--gb") {
                Some(Accent::Gb)
            } else if args.iter().any(|a| a == "--us") {
                Some(Accent::Us)
            } else {
                None
            };
            let language = option(args, "--language").map(str::to_lowercase);
            if let Some(language) = &language
                && !languages().contains(&language.as_str())
            {
                return Err(format!("--language {language}: one of {}", languages().join(", ")));
            }
            let found = alikes(
                &word,
                &AlikeOptions {
                    accent,
                    level,
                    kinds,
                    most,
                    language,
                },
            );
            if found.is_empty() {
                println!("none");
            }
            for alike in found {
                let kind = format!("{:?}", alike.kind).to_lowercase();
                println!(
                    "{:<12} {kind:<7} {:.2}  {:<12} {}",
                    alike.word, alike.distance, alike.sound, alike.why
                );
            }
            return Ok(());
        }
        Some("level") => {
            for word in args.iter().skip(1) {
                let level = word_level(word).map_or("not in the lists".to_string(), |level| format!("{level:?}"));
                println!("{word}: {level}");
            }
            return Ok(());
        }
        _ => {}
    }
    let usage = "usage: sanjeh (pronounce | melts | level | alike) …, or sanjeh line (features|measure|bench) <file.jsonl | -> [--model <file>] [--split fit|test|all]";
    let (Some("line"), Some(command), Some(path)) = (
        args.first().map(String::as_str),
        args.get(1).map(String::as_str),
        args.get(2),
    ) else {
        return Err(usage.into());
    };
    match command {
        "features" => {
            for item in read_items(path)? {
                let f = facts(&item.input);
                let row = serde_json::json!({
                    "id": item.id, "group": item.group, "split": item.split, "features": f.features,
                    "usable": { "ear": f.why_not(Skill::Ear).is_empty(), "mouth": f.why_not(Skill::Mouth).is_empty() },
                    "labels": item.labels,
                });
                println!("{row}");
            }
        }
        "measure" => {
            let model = read_model(args)?;
            let text = read_text(path)?;
            for line in text.lines().filter(|l| !l.trim().is_empty()) {
                let input: LineInput = serde_json::from_str::<Item>(line)
                    .map(|item| item.input)
                    .or_else(|_| serde_json::from_str(line))
                    .map_err(|e| format!("{path}: {e}"))?;
                println!(
                    "{}",
                    serde_json::to_string(&measure(&input, &model)).map_err(|e| e.to_string())?
                );
            }
        }
        "bench" => {
            let model = read_model(args)?;
            let split = option(args, "--split").unwrap_or("test");
            let items: Vec<_> = read_items(path)?
                .into_iter()
                .filter(|item| split == "all" || item.split == split)
                .map(|item| {
                    let m = measure(&item.input, &model);
                    (item, m)
                })
                .collect();
            println!("{} on {path} ({split}: {} lines)", model.name, items.len());
            for skill in [Skill::Ear, Skill::Mouth] {
                let r = report(&items, skill);
                let pct = |x: Option<f64>| x.map_or("—".to_string(), |x| format!("{:.0}%", x * 100.0));
                let two = |x: Option<f64>| x.map_or("—".to_string(), |x| format!("{x:.2}"));
                println!(
                    "  {:5}  {} rated · votes matched {} · level AUC {} · worth AUC {} · at their level, right {} · called worth it, yes {}",
                    format!("{skill:?}").to_lowercase(),
                    r.rated,
                    pct(r.votes_matched),
                    two(r.level_auc),
                    two(r.worth_auc),
                    pct(r.at_level_right),
                    pct(r.worth_yes),
                );
                let by: Vec<String> = r
                    .by_level
                    .iter()
                    .map(|(l, n, hit)| format!("{l:?} {n}: {:.0}%", hit * 100.0))
                    .collect();
                println!("         by the level it gave: {}", by.join(" · "));
            }
        }
        _ => return Err(usage.into()),
    }
    Ok(())
}
