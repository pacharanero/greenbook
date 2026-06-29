use chrono::{Local, NaiveDate};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use greenbook::evaluate::{OverallStatus, SeriesCompletionStatus, VaccinationStatus};
use greenbook::{
    evaluate, load_effective_schedule_for_date, load_product_map, load_schedule,
    load_schedule_versions, parse_fhir_bundle, ScheduleSelection,
};
use std::fs;
use std::io::IsTerminal;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "greenbook",
    version,
    about = "UK childhood immunisation schedule evaluator (POC)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Evaluate a FHIR bundle against a schedule.
    Evaluate {
        /// Path to a schedule TOML file.
        schedule: PathBuf,
        /// Path to a product mapping TOML file.
        products: PathBuf,
        /// Path to a FHIR Bundle JSON file.
        bundle: PathBuf,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Report)]
        format: OutputFormat,
        /// Override the evaluation date (defaults to today).
        #[arg(long)]
        evaluated_at: Option<NaiveDate>,
    },
    /// Evaluate a FHIR bundle using schedule versions selected from a rules directory.
    EvaluateAuto {
        /// Directory containing schedule-<country>-*.toml files.
        rules_dir: PathBuf,
        /// Path to a product mapping TOML file.
        products: PathBuf,
        /// Path to a FHIR Bundle JSON file.
        bundle: PathBuf,
        /// Jurisdiction code.
        #[arg(long, default_value = "UK")]
        country: String,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Report)]
        format: OutputFormat,
        /// Override the evaluation date (defaults to today).
        #[arg(long)]
        evaluated_at: Option<NaiveDate>,
        /// Include schedule-selection rationale in report output.
        #[arg(long)]
        verbose: bool,
    },
    /// List available schedule versions in a rules directory.
    Versions {
        /// Directory containing schedule-<country>-*.toml files.
        rules_dir: PathBuf,
        /// Jurisdiction code.
        #[arg(long, default_value = "UK")]
        country: String,
    },
    /// Generate or install shell completions.
    Completions {
        #[command(subcommand)]
        command: Option<CompletionCommand>,
        /// Shell to generate completions for.
        #[arg(value_enum)]
        shell: Option<Shell>,
        /// Output directory. Prints to stdout when omitted.
        #[arg(long, short = 'd')]
        dir: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum CompletionCommand {
    /// Install completions for the current user.
    Install {
        /// Shell to install completions for. Detected from $SHELL when omitted.
        #[arg(long)]
        shell: Option<Shell>,
        /// Completion directory to write to.
        #[arg(long, short = 'd')]
        dir: Option<PathBuf>,
    },
}

#[derive(Copy, Clone, Debug, clap::ValueEnum)]
enum OutputFormat {
    /// Full machine-readable result (the complete evaluation).
    Json,
    /// Human-readable per-series breakdown.
    Report,
    /// Just the headline answer, in one coloured line.
    Status,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Command::Evaluate {
            schedule,
            products,
            bundle,
            format,
            evaluated_at,
        } => {
            let schedule = load_schedule(&schedule)?;
            let products = load_product_map(&products)?;
            let bundle_json = fs::read_to_string(&bundle)?;
            let record = parse_fhir_bundle(&bundle_json)?;
            let evaluated_at = evaluated_at.unwrap_or_else(|| Local::now().date_naive());
            let status = evaluate(&record, &schedule, &products, evaluated_at)?;

            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&status)?);
                }
                OutputFormat::Report => {
                    print_report(&status, &record, None);
                }
                OutputFormat::Status => {
                    print_status(&status);
                }
            }
            Ok(())
        }
        Command::EvaluateAuto {
            rules_dir,
            products,
            bundle,
            country,
            format,
            evaluated_at,
            verbose,
        } => {
            let products = load_product_map(&products)?;
            let bundle_json = fs::read_to_string(&bundle)?;
            let record = parse_fhir_bundle(&bundle_json)?;
            let evaluated_at = evaluated_at.unwrap_or_else(|| Local::now().date_naive());
            let historical =
                load_effective_schedule_for_date(&rules_dir, &country, record.dob, evaluated_at)?;
            let status = evaluate(&record, &historical.schedule, &products, evaluated_at)?;

            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&status)?);
                }
                OutputFormat::Report => {
                    print_report(&status, &record, verbose.then_some(&historical.selection));
                }
                OutputFormat::Status => {
                    print_status(&status);
                }
            }
            Ok(())
        }
        Command::Versions { rules_dir, country } => {
            let versions = load_schedule_versions(&rules_dir, &country)?;
            for version in versions {
                let meta = &version.schedule.schedule;
                let effective_to = version
                    .effective_to
                    .map(|d| d.to_string())
                    .unwrap_or_else(|| "open".into());
                println!(
                    "{}  effective_to={}  {}  {}",
                    meta.valid_from,
                    effective_to,
                    version.path.display(),
                    meta.source_document
                );
                if let Some(summary) = &meta.change_summary {
                    println!("    {}", summary.trim());
                }
            }
            Ok(())
        }
        Command::Completions {
            command,
            shell,
            dir,
        } => run_completions(command, shell, dir.as_deref()),
    }
}

fn run_completions(
    command: Option<CompletionCommand>,
    shell: Option<Shell>,
    dir: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Cli::command();
    match command {
        Some(CompletionCommand::Install { shell, dir }) => {
            let shell = shell.or_else(detect_shell).ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    "could not detect shell; pass --shell bash|zsh|fish|powershell|elvish",
                )
            })?;
            let dir = dir
                .map(Ok)
                .unwrap_or_else(|| default_completion_dir(shell))?;
            write_completion(shell, &mut cmd, &dir)?;
            print_install_note(shell, &dir);
        }
        None => {
            let shell = shell.ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    "missing shell; try `greenbook completions install`",
                )
            })?;
            if let Some(dir) = dir {
                write_completion(shell, &mut cmd, dir)?;
            } else {
                generate(shell, &mut cmd, "greenbook", &mut std::io::stdout());
            }
        }
    }
    Ok(())
}

fn write_completion(
    shell: Shell,
    cmd: &mut clap::Command,
    dir: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    fs::create_dir_all(dir)?;
    let path = dir.join(completion_filename(shell));
    let mut file = fs::File::create(&path)?;
    generate(shell, cmd, "greenbook", &mut file);
    println!("Completion script written to: {}", path.display());
    Ok(path)
}

fn completion_filename(shell: Shell) -> &'static str {
    match shell {
        Shell::Bash => "greenbook",
        Shell::Zsh => "_greenbook",
        Shell::Fish => "greenbook.fish",
        Shell::PowerShell => "greenbook.ps1",
        Shell::Elvish => "greenbook.elv",
        _ => "greenbook.completion",
    }
}

fn detect_shell() -> Option<Shell> {
    let shell = std::env::var("SHELL").ok()?;
    let name = Path::new(&shell).file_name()?.to_string_lossy();
    match name.as_ref() {
        "bash" => Some(Shell::Bash),
        "zsh" => Some(Shell::Zsh),
        "fish" => Some(Shell::Fish),
        "elvish" => Some(Shell::Elvish),
        _ => None,
    }
}

fn default_completion_dir(shell: Shell) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let home = home_dir()
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "could not determine home directory"))?;
    Ok(match shell {
        Shell::Bash => std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/share"))
            .join("bash-completion/completions"),
        Shell::Zsh => home.join(".zfunc"),
        Shell::Fish => std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"))
            .join("fish/completions"),
        Shell::PowerShell => home.join(".config/powershell/completions"),
        Shell::Elvish => home.join(".elvish/lib"),
        _ => home.join(".local/share/greenbook/completions"),
    })
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn print_install_note(shell: Shell, dir: &Path) {
    match shell {
        Shell::Zsh => {
            println!("Add this before `compinit` in ~/.zshrc if it is not already there:");
            println!("  fpath=({} $fpath)", dir.display());
            println!("Then restart zsh or run `autoload -Uz compinit && compinit`.");
        }
        Shell::PowerShell => {
            println!("Add this to your PowerShell profile if it is not already there:");
            println!("  . {}/greenbook.ps1", dir.display());
        }
        _ => println!("Restart your shell to load the updated completions."),
    }
}

/// The headline answer, distilled to a single coloured line - the "traffic
/// light" view. Green when up to date for age, red when behind, amber when the
/// record is too sparse to judge. Colour is ANSI, suppressed when stdout is not
/// a terminal or `NO_COLOR` is set, so piping (`| cat`, into a file) stays clean.
fn print_status(status: &VaccinationStatus) {
    let (colour, text) = match status.status {
        OverallStatus::UpToDateForAge => ("32", "Up to date for age"),
        OverallStatus::BehindForAge | OverallStatus::Unvaccinated => {
            ("31", "Not up to date for age")
        }
        OverallStatus::Unknown => ("33", "Status unknown"),
    };
    if std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none() {
        println!("\x1b[1;{colour}m{text}\x1b[0m");
    } else {
        println!("{text}");
    }
}

fn print_report(
    status: &VaccinationStatus,
    record: &greenbook::VaccinationRecord,
    selection: Option<&ScheduleSelection>,
) {
    println!("Greenbook evaluation");
    println!("====================");
    if let Some(id) = &record.patient_id {
        println!("Patient:           {}", id);
    }
    println!("DOB:               {}", record.dob);
    println!("Evaluated at:      {}", status.evaluated_at);
    println!("Schedule version:  {}", status.schedule_version);
    if let Some(selection) = selection {
        println!("Schedule rule:     {}", selection.rule);
        println!("Schedule versions:");
        for version in &selection.versions {
            let effective_to = version
                .effective_to
                .map(|d| d.to_string())
                .unwrap_or_else(|| "open".into());
            println!(
                "  - {} to {}  {}",
                version.valid_from, effective_to, version.source_document
            );
        }
    }
    // The headline answer is age-relative ("are there gaps that should be filled
    // by now?"). The strict flag answers the separate "had everything ever?".
    println!("Up-to-date status: {}", overall_label(status.status));
    println!(
        "Fully vaccinated:  {} (strict: every eligible series complete)",
        if status.fully_vaccinated { "yes" } else { "no" }
    );
    println!();
    println!("By series:");
    println!("---------");
    for s in &status.by_series {
        // Show valid-of-due so the reader sees progress against what is *due*,
        // with the series total in parentheses.
        let age_note = if s.eligible && s.up_to_date_for_age {
            "up to date"
        } else if s.eligible {
            "BEHIND"
        } else {
            "not applicable"
        };
        println!(
            "  [{}] {} ({}/{} due, {} total) - {}",
            series_label(s.status),
            s.display_name,
            s.doses_valid,
            s.doses_due,
            s.doses_expected,
            age_note,
        );
        for n in &s.notes {
            println!("      note: {}", n);
        }
        for d in &s.doses_recorded {
            // "ok" = within the standard schedule; "OUT-OF-SCHEDULE" = given but
            // too early/late/short-interval, so it does not count (see §5).
            let mark = if d.within_schedule {
                "ok            "
            } else {
                "OUT-OF-SCHEDULE"
            };
            let dose_n = d
                .assigned_dose_number
                .map(|n| format!("dose {}", n))
                .unwrap_or_else(|| "unassigned".into());
            println!(
                "      - {}  {}  {}  ({})  [{}]",
                mark,
                d.date,
                dose_n,
                d.age_at_dose,
                d.display.as_deref().unwrap_or(&d.vaccine_code),
            );
            for r in &d.schedule_notes {
                println!("          ! {}", r);
            }
            for f in &d.flags {
                println!("          ? {}", f);
            }
        }
    }

    // Doses that matched no series at all - surfaced rather than dropped.
    if !status.unmatched_doses.is_empty() {
        println!();
        println!("Unmatched doses:");
        println!("---------------");
        for u in &status.unmatched_doses {
            println!(
                "  - {}  [{}]  ({})",
                u.date,
                u.display.as_deref().unwrap_or(&u.vaccine_code),
                u.reason,
            );
        }
    }

    // Likely duplicate "echoes" - same procedure code as an earlier dose.
    if !status.duplicate_doses.is_empty() {
        println!();
        println!("Duplicate doses:");
        println!("---------------");
        for dup in &status.duplicate_doses {
            println!(
                "  - {}  [{}]  (likely duplicate of {}; same procedure code)",
                dup.date,
                dup.display.as_deref().unwrap_or(&dup.vaccine_code),
                dup.duplicate_of,
            );
        }
    }
}

fn overall_label(o: OverallStatus) -> &'static str {
    match o {
        OverallStatus::UpToDateForAge => "UP_TO_DATE_FOR_AGE",
        OverallStatus::BehindForAge => "BEHIND_FOR_AGE",
        OverallStatus::Unvaccinated => "UNVACCINATED",
        OverallStatus::Unknown => "UNKNOWN",
    }
}

fn series_label(s: SeriesCompletionStatus) -> &'static str {
    match s {
        SeriesCompletionStatus::Complete => "COMPLETE   ",
        SeriesCompletionStatus::Partial => "PARTIAL    ",
        SeriesCompletionStatus::None => "NONE       ",
        SeriesCompletionStatus::NotApplicable => "N/A        ",
    }
}
