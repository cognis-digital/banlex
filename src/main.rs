//! `banlex` command-line interface.
//!
//! Subcommands:
//!   * `scan` — score text (positional or `--file`) and print a verdict.
//!   * `categories` — list categories in the active lexicon.
//!
//! Exit codes for `scan`:
//!   * `0` — CLEAR (total score below threshold)
//!   * `1` — FLAGGED (total score >= threshold)
//!   * `2` — usage / IO / lexicon error

use banlex::{default_lexicon, Lexicon, Scanner};
use std::process::ExitCode;

const DEFAULT_THRESHOLD: f64 = 1.0;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

fn run(args: &[String]) -> Result<ExitCode, String> {
    let cmd = match args.first().map(|s| s.as_str()) {
        Some("scan") => "scan",
        Some("categories") => "categories",
        Some("-h") | Some("--help") | None => {
            print_help();
            return Ok(ExitCode::SUCCESS);
        }
        Some("-V") | Some("--version") => {
            println!("banlex {}", env!("CARGO_PKG_VERSION"));
            return Ok(ExitCode::SUCCESS);
        }
        Some(other) => return Err(format!("unknown command '{other}' (try --help)")),
    };

    // Parse shared/sub options.
    let mut json_out = false;
    let mut lexicon_path: Option<String> = None;
    let mut threshold = DEFAULT_THRESHOLD;
    let mut file_path: Option<String> = None;
    let mut positional: Vec<String> = Vec::new();

    let mut it = args[1..].iter().peekable();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--json" => json_out = true,
            "--lexicon" => {
                lexicon_path = Some(
                    it.next()
                        .ok_or("--lexicon requires a path")?
                        .clone(),
                );
            }
            "--threshold" => {
                let v = it.next().ok_or("--threshold requires a number")?;
                threshold = v
                    .parse::<f64>()
                    .map_err(|_| format!("invalid --threshold value '{v}'"))?;
                if !threshold.is_finite() || threshold < 0.0 {
                    return Err("--threshold must be finite and non-negative".into());
                }
            }
            "--file" => {
                file_path = Some(it.next().ok_or("--file requires a path")?.clone());
            }
            "-h" | "--help" => {
                print_help();
                return Ok(ExitCode::SUCCESS);
            }
            s if s.starts_with('-') => return Err(format!("unknown option '{s}'")),
            other => positional.push(other.to_string()),
        }
    }

    let lexicon = load_lexicon(lexicon_path.as_deref())?;

    match cmd {
        "categories" => {
            let cats = lexicon.categories();
            if json_out {
                let v = serde_json::json!({
                    "lexicon": lexicon.name,
                    "categories": cats,
                });
                println!("{}", serde_json::to_string_pretty(&v).unwrap());
            } else {
                println!("Lexicon: {} ({} entries)", lexicon.name, lexicon.len());
                println!("Categories:");
                for c in cats {
                    let n = lexicon
                        .entries
                        .iter()
                        .filter(|e| e.category == c)
                        .count();
                    println!("  {c}  ({n} terms)");
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        "scan" => {
            let text = resolve_text(&positional, file_path.as_deref())?;
            let scanner = Scanner::new(lexicon, threshold);
            let result = scanner.scan(&text);

            if json_out {
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            } else {
                print_human(&result);
            }

            if result.flagged {
                Ok(ExitCode::from(1))
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
        _ => unreachable!(),
    }
}

fn load_lexicon(path: Option<&str>) -> Result<Lexicon, String> {
    match path {
        None => Ok(default_lexicon()),
        Some(p) => {
            let raw = std::fs::read_to_string(p)
                .map_err(|e| format!("cannot read lexicon '{p}': {e}"))?;
            Lexicon::from_json(&raw).map_err(|e| e.to_string())
        }
    }
}

fn resolve_text(positional: &[String], file: Option<&str>) -> Result<String, String> {
    if let Some(f) = file {
        return std::fs::read_to_string(f).map_err(|e| format!("cannot read file '{f}': {e}"));
    }
    if positional.is_empty() {
        return Err("scan requires text (positional) or --file <path>".into());
    }
    Ok(positional.join(" "))
}

fn print_human(r: &banlex::ScanResult) {
    println!("verdict: {}", r.verdict());
    println!(
        "total score: {:.3} (threshold {:.3}, {} tokens)",
        r.total_score, r.threshold, r.token_count
    );
    if r.category_scores.is_empty() {
        println!("categories: (none matched)");
    } else {
        println!("categories:");
        for c in &r.category_scores {
            println!("  {:<22} {:.3}  ({} matches)", c.category, c.score, c.match_count);
        }
    }
    if !r.matches.is_empty() {
        println!("matched terms:");
        for m in &r.matches {
            let lang = m.lang.as_deref().unwrap_or("-");
            println!(
                "  [{}] \"{}\" (w={:.2}, lang={}, @tok {})",
                m.category, m.term, m.weight, lang, m.start_token
            );
        }
    }
}

fn print_help() {
    println!(
        r#"banlex {ver} - multilingual content-risk lexicon and classifier

USAGE:
    banlex scan [OPTIONS] [TEXT]...
    banlex categories [OPTIONS]

COMMANDS:
    scan          Score text against the lexicon and print a verdict
    categories    List the categories in the active lexicon

OPTIONS:
    --lexicon <PATH>     Load a custom lexicon JSON (default: built-in)
    --threshold <N>      Flag threshold for the verdict (default: {thr})
    --file <PATH>        Read scan text from a file instead of arguments
    --json               Emit machine-readable JSON
    -h, --help           Show this help
    -V, --version        Show version

EXIT CODES (scan):
    0  CLEAR     total score below threshold
    1  FLAGGED   total score >= threshold
    2  ERROR     usage / IO / lexicon error

EXAMPLES:
    banlex scan "you have won, send gift card now"
    banlex scan --file message.txt --threshold 1.5 --json
    banlex categories --lexicon examples/lexicon.json
"#,
        ver = env!("CARGO_PKG_VERSION"),
        thr = DEFAULT_THRESHOLD
    );
}
