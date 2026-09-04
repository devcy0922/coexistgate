use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use coexistgate_core::{
    analyze, render_human, rule_catalog, AnalysisRequest, FileTree, GateDecision, Policy, Severity,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

#[derive(Parser, Debug)]
#[command(
    name = "coexistgate",
    about = "Release safety checks your code review doesn't cover.",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Compare previous and candidate trees and emit a release gate
    Analyze {
        /// Repository path (candidate working tree)
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Git ref for the previous release (default: origin/main, origin/master, main, master)
        #[arg(long)]
        base: Option<String>,
        /// Git ref for the candidate (default: working tree)
        #[arg(long)]
        head: Option<String>,
        /// Explicit previous tree directory (fixture mode; skips git)
        #[arg(long)]
        base_dir: Option<PathBuf>,
        /// Explicit candidate tree directory (fixture mode)
        #[arg(long)]
        head_dir: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
        /// Override policy gate.fail_on (repeatable or comma-separated)
        #[arg(long, value_delimiter = ',')]
        fail_on: Vec<String>,
        #[arg(long)]
        policy: Option<PathBuf>,
    },
    /// List built-in rules
    Rules {
        #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
    },
    /// Explain one rule
    Explain { rule_id: String },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    Human,
    Json,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("coexistgate: {err:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Analyze {
            path,
            base,
            head,
            base_dir,
            head_dir,
            format,
            fail_on,
            policy,
        } => {
            let fail_on = parse_fail_on(&fail_on)?;
            let (previous, candidate) = if let (Some(b), Some(h)) = (&base_dir, &head_dir) {
                (load_dir(b)?, load_dir(h)?)
            } else if let Some(h) = &head_dir {
                let prev = if let Some(b) = &base_dir {
                    load_dir(b)?
                } else {
                    FileTree::new()
                };
                (prev, load_dir(h)?)
            } else {
                load_git_or_dir(&path, base.as_deref(), head.as_deref())?
            };
            let policy = match policy {
                Some(p) => Some(Policy::from_yaml(&fs::read_to_string(p)?)?),
                None => None,
            };
            let report = analyze(AnalysisRequest {
                previous,
                candidate,
                policy,
                fail_on,
            })?;
            match format {
                OutputFormat::Human => print!("{}", render_human(&report)),
                OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
            }
            Ok(match report.gate {
                GateDecision::Pass => ExitCode::SUCCESS,
                GateDecision::Fail => ExitCode::from(1),
            })
        }
        Commands::Rules { format } => {
            let rules = rule_catalog();
            match format {
                OutputFormat::Human => {
                    println!("{:<28} {:<8} {:<16} CROSS", "ID", "SEV", "CATEGORY");
                    for r in rules {
                        println!(
                            "{:<28} {:<8} {:<16} {:<5} {}",
                            r.id,
                            r.default_severity.as_str(),
                            r.category.as_str(),
                            if r.cross_artifact { "yes" } else { "no" },
                            r.title
                        );
                    }
                }
                OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&json_rules())?),
            }
            Ok(ExitCode::SUCCESS)
        }
        Commands::Explain { rule_id } => {
            let Some(r) = rule_catalog()
                .into_iter()
                .find(|r| r.id.eq_ignore_ascii_case(&rule_id))
            else {
                bail!("unknown rule: {rule_id}");
            };
            println!("{}\n", r.id);
            println!("{}\n", r.title);
            println!("severity: {}", r.default_severity.as_str());
            println!("category: {}", r.category.as_str());
            println!(
                "cross-artifact: {}",
                if r.cross_artifact { "yes" } else { "no" }
            );
            println!("\n{}", r.explanation);
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn json_rules() -> Vec<serde_json::Value> {
    rule_catalog()
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "title": r.title,
                "severity": r.default_severity.as_str(),
                "category": r.category.as_str(),
                "cross_artifact": r.cross_artifact,
                "explanation": r.explanation,
            })
        })
        .collect()
}

fn parse_fail_on(items: &[String]) -> Result<Option<Vec<Severity>>> {
    if items.is_empty() {
        return Ok(None);
    }
    let mut out = Vec::new();
    for s in items {
        let Some(sev) = Severity::parse(s) else {
            bail!("unknown severity for --fail-on: {s}");
        };
        out.push(sev);
    }
    Ok(Some(out))
}

fn load_git_or_dir(
    path: &Path,
    base: Option<&str>,
    head: Option<&str>,
) -> Result<(FileTree, FileTree)> {
    if is_git_repo(path) {
        let base_ref = match base {
            Some(b) => Some(b.to_string()),
            None => default_base(path),
        };
        let previous = match &base_ref {
            Some(b) => load_git_tree(path, b)?,
            None => FileTree::new(),
        };
        let candidate = match head {
            Some(h) => load_git_tree(path, h)?,
            None => load_dir(path)?,
        };
        if base_ref.is_none() {
            eprintln!(
                "coexistgate: no base ref found; previous tree is empty (candidate-only rules still run)"
            );
        }
        Ok((previous, candidate))
    } else {
        if base.is_some() || head.is_some() {
            bail!("--base/--head require a git repository");
        }
        Ok((FileTree::new(), load_dir(path)?))
    }
}

fn is_git_repo(path: &Path) -> bool {
    Command::new("git")
        .args([
            "-C",
            &path.to_string_lossy(),
            "rev-parse",
            "--is-inside-work-tree",
        ])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn default_base(path: &Path) -> Option<String> {
    for cand in ["origin/main", "origin/master", "main", "master"] {
        if ref_exists(path, cand) {
            return Some(cand.to_string());
        }
    }
    None
}

fn ref_exists(path: &Path, git_ref: &str) -> bool {
    Command::new("git")
        .args([
            "-C",
            &path.to_string_lossy(),
            "rev-parse",
            "--verify",
            git_ref,
        ])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn load_git_tree(path: &Path, git_ref: &str) -> Result<FileTree> {
    let output = Command::new("git")
        .args([
            "-C",
            &path.to_string_lossy(),
            "ls-tree",
            "-r",
            "--name-only",
            git_ref,
        ])
        .output()
        .context("git ls-tree")?;
    if !output.status.success() {
        bail!(
            "git ls-tree {git_ref} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let mut tree = FileTree::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let rel = line.trim();
        if rel.is_empty() || skip_path(rel) {
            continue;
        }
        let show = Command::new("git")
            .args([
                "-C",
                &path.to_string_lossy(),
                "show",
                &format!("{git_ref}:{rel}"),
            ])
            .output()
            .with_context(|| format!("git show {git_ref}:{rel}"))?;
        if !show.status.success() {
            continue;
        }
        if show.stdout.len() > 1024 * 1024 {
            continue;
        }
        if let Ok(text) = String::from_utf8(show.stdout) {
            tree.insert(rel, text).ok();
        }
    }
    Ok(tree)
}

fn load_dir(root: &Path) -> Result<FileTree> {
    let mut tree = FileTree::new();
    walk(root, root, &mut tree)?;
    Ok(tree)
}

fn walk(root: &Path, dir: &Path, tree: &mut FileTree) -> Result<()> {
    let entries = fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(root).unwrap_or(&path);
        let rel_s = rel.to_string_lossy().replace('\\', "/");
        if skip_path(&rel_s) {
            continue;
        }
        if path.is_dir() {
            walk(root, &path, tree)?;
        } else if path.is_file() {
            let meta = fs::metadata(&path)?;
            if meta.len() > 1024 * 1024 {
                continue;
            }
            match fs::read_to_string(&path) {
                Ok(text) => {
                    tree.insert(rel_s, text).ok();
                }
                Err(_) => continue,
            }
        }
    }
    Ok(())
}

fn skip_path(rel: &str) -> bool {
    rel.split('/').any(|seg| {
        matches!(
            seg,
            ".git" | "node_modules" | "target" | "dist" | "vendor" | ".next" | "coverage"
        )
    })
}
