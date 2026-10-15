//! `sanjeh`, the measures from the command line.
//!
//! ```text
//! sanjeh line features <items.jsonl>                 each item's features, for fitting a model
//! sanjeh line measure  <items.jsonl> --model <file>  each item measured
//! sanjeh line bench    <items.jsonl> --model <file> [--split test]
//! ```
//!
//! An items file is a benchmark (`sanjeh::bench`); `measure` also takes
//! bare line inputs, one a line.

use std::process::ExitCode;

use sanjeh::bench::{Item, report};
use sanjeh::line::{LineInput, LineModel, Skill, facts, measure};

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

fn read_items(path: &str) -> Result<Vec<Item>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .enumerate()
        .map(|(at, l)| serde_json::from_str(l).map_err(|e| format!("{path}:{}: {e}", at + 1)))
        .collect()
}

fn read_model(args: &[String]) -> Result<LineModel, String> {
    let path = option(args, "--model").ok_or("--model <file> is needed")?;
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    LineModel::from_json(&text).map_err(|e| format!("{path}: {e}"))
}

fn run(args: &[String]) -> Result<(), String> {
    let usage = "usage: sanjeh line (features|measure|bench) <items.jsonl> [--model <file>] [--split fit|test]";
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
            let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
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
