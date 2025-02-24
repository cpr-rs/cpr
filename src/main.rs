mod config;
mod errors;
mod format;
mod subcommands;

use clap::{Parser, Subcommand};
use config::Config;
use miette::IntoDiagnostic;
use simple_logger::SimpleLogger;
use std::path::PathBuf;
use subcommands::{init, new, prompt_project_info};

pub fn get_styles() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .usage(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))),
        )
        .header(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))),
        )
        .literal(
            anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green))),
        )
        .invalid(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red))),
        )
        .error(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red))),
        )
        .valid(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green))),
        )
        .placeholder(
            anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::White))),
        )
}

#[derive(Debug, Parser)]
#[command(name = "cpr")]
#[command(version, about = "A simple git-based project manager aimed at C/C++", long_about = None, styles = get_styles())]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, help = "Global configuration file path")]
    config: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Initialize a directory with a template from GitHub
    #[command(arg_required_else_help = true)]
    Init {
        /// Directory to target (ex. ./my_project)
        directory: PathBuf,
        /// Repository path optionally including prefix (ex. gh:cpr-rs/cpp, cpr-rs/cpp)
        repo_path: String,
    },
    /// Create a new project with a template from GitHub
    #[command(arg_required_else_help = true)]
    New {
        /// Repository path from GitHub (ex. gh:cpr-rs/cpp, cpr-rs/cpp)
        repo_path: String,
    },
}

fn main() -> miette::Result<()> {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Error)
        .env()
        .init()
        .into_diagnostic()?;

    let args = Cli::parse();

    let config_path = args.config.unwrap_or_else(|| {
        dirs::home_dir()
            .map(|d| d.join(".cpr").join("config.toml"))
            .expect("Could not determine home directory")
    });
    if !config_path.exists() {
        Config::init(&config_path)?;
        println!("Created default configuration at {:?}", config_path);
    }

    let config = Config::from_file(config_path)?;

    match args.command {
        Commands::Init {
            directory,
            repo_path,
        } => {
            init(directory, repo_path, prompt_project_info(&config)?)?;
        }
        Commands::New { repo_path } => {
            new(repo_path, prompt_project_info(&config)?)?;
        }
    }

    Ok(())
}
