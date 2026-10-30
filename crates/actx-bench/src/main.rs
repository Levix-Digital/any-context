//! `actx-bench`: development benchmarks (contract: specs/001-retrieval-benchmark/contracts/actx-bench-cli.md).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use actx_bench::compare::{compare, summary, Comparison};
use actx_bench::result::RunResult;
use actx_bench::runner::{self, RunError, RunOptions};

const EXIT_REGRESSION: u8 = 1;
const EXIT_LABELS: u8 = 2;
const EXIT_ENVIRONMENT: u8 = 3;
const EXIT_NOT_COMPARABLE: u8 = 4;
const EXIT_NON_DETERMINISTIC: u8 = 5;

#[derive(Parser)]
#[command(name = "actx-bench", about = "AnyContext development benchmarks")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Retrieval quality on the fixed corpus in benchmarks/retrieval.
    Retrieval {
        #[command(subcommand)]
        action: Action,
        /// Benchmark folder (corpus/, questions.json, baselines).
        #[arg(long, global = true, default_value = "benchmarks/retrieval")]
        dir: PathBuf,
    },
}

#[derive(Subcommand)]
enum Action {
    /// Check the question labels against the corpus.
    Validate,
    /// Index the corpus in a sandbox and run every question.
    Run {
        /// Embedding model for a dense run: a catalog id, optionally `<provider>/<id>` (e.g.
        /// `local/multilingual-e5-small`; a cloud model takes its key from the environment).
        #[arg(long)]
        embedding: Option<String>,
        /// Where the reranker is cached (default: ACTX_MODELS_DIR, else a temp folder).
        #[arg(long)]
        models_dir: Option<PathBuf>,
        /// Reranker of the run: a catalog id (default: mmarco-reranker).
        #[arg(long)]
        reranker: Option<String>,
        /// Result file (default: <dir>/results/<config_id>.json).
        #[arg(long)]
        out: Option<PathBuf>,
        /// Baseline to compare with; when given, a missing or not comparable baseline fails.
        #[arg(long)]
        compare: Option<PathBuf>,
        /// Save the result as the baseline of its configuration.
        #[arg(long)]
        save_baseline: bool,
        /// Run the ranking pass twice and fail on any difference.
        #[arg(long)]
        check_determinism: bool,
        /// Also write the report as Markdown to this file (appended).
        #[arg(long)]
        summary: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    let Command::Retrieval { action, dir } = Cli::parse().command;
    let corpus_dir = dir.join("corpus");
    let questions_file = dir.join("questions.json");
    match action {
        Action::Validate => match runner::load_and_validate(&corpus_dir, &questions_file) {
            Ok((corpus, file, _)) => {
                println!("{} questions valid against {} documents", file.questions.len(), corpus.len());
                ExitCode::SUCCESS
            }
            Err(e) => report_error(e),
        },
        Action::Run { embedding, models_dir, reranker, out, compare: compare_with, save_baseline, check_determinism, summary: summary_file } => {
            let models_dir = models_dir.or_else(|| std::env::var_os("ACTX_MODELS_DIR").map(PathBuf::from));
            let options = RunOptions { corpus_dir, questions_file, embedding, models_dir, reranker, check_determinism };
            let rt = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => return environment(&format!("tokio runtime: {e}")),
            };
            let result = match rt.block_on(runner::run(&options)) {
                Ok(r) => r,
                Err(RunError::Environment(e)) if e.starts_with("non-deterministic") => {
                    eprintln!("error: {e}");
                    return ExitCode::from(EXIT_NON_DETERMINISTIC);
                }
                Err(e) => return report_error(e),
            };
            finish(&dir, &result, out, compare_with, save_baseline, summary_file)
        }
    }
}

fn finish(
    dir: &Path,
    result: &RunResult,
    out: Option<PathBuf>,
    compare_with: Option<PathBuf>,
    save_baseline: bool,
    summary_file: Option<PathBuf>,
) -> ExitCode {
    let id = &result.config.config_id;
    let out = out.unwrap_or_else(|| dir.join("results").join(format!("{id}.json")));
    if let Err(e) = result.write(&out) {
        return environment(&e);
    }
    let explicit = compare_with.is_some();
    let baseline_path = compare_with.unwrap_or_else(|| dir.join(format!("baseline.{id}.json")));
    let comparison = if baseline_path.exists() {
        match RunResult::read(&baseline_path) {
            Ok(baseline) => Some(compare(&baseline, result)),
            Err(e) => return environment(&e),
        }
    } else {
        None
    };

    let report = summary(result, comparison.as_ref());
    println!("{report}");
    println!("result: {}", out.display());
    if let Some(path) = summary_file {
        use std::io::Write;
        let written = std::fs::OpenOptions::new().create(true).append(true).open(&path).and_then(|mut f| f.write_all(report.as_bytes()));
        if let Err(e) = written {
            return environment(&format!("'{}' cannot be written: {e}", path.display()));
        }
    }
    if save_baseline {
        let path = dir.join(format!("baseline.{id}.json"));
        if let Err(e) = result.write(&path) {
            return environment(&e);
        }
        println!("baseline saved: {}", path.display());
        return ExitCode::SUCCESS;
    }
    match comparison {
        Some(c) if c.failed() => ExitCode::from(EXIT_REGRESSION),
        Some(Comparison::NotComparable { .. }) if explicit => ExitCode::from(EXIT_NOT_COMPARABLE),
        None if explicit => {
            eprintln!("error: no baseline at '{}'; create it with `run --save-baseline` and commit it", baseline_path.display());
            ExitCode::from(EXIT_NOT_COMPARABLE)
        }
        _ => ExitCode::SUCCESS,
    }
}

fn report_error(e: RunError) -> ExitCode {
    match e {
        RunError::Labels(errors) => {
            for err in &errors {
                eprintln!("label error: {err}");
            }
            eprintln!("{} label errors", errors.len());
            ExitCode::from(EXIT_LABELS)
        }
        RunError::Environment(e) => environment(&e),
    }
}

fn environment(e: &str) -> ExitCode {
    eprintln!("error: {e}");
    ExitCode::from(EXIT_ENVIRONMENT)
}
