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
    /// Initialize a directory with a template
    #[command(arg_required_else_help = true)]
    Init {
        /// Directory to target (ex. ./my_project)
        directory: PathBuf,
        /// Repository path optionally including prefix (ex. gh:cpr-rs/cpp, cpr-rs/cpp)
        repo_path: String,
    },
    /// Create a new project with a template
    #[command(arg_required_else_help = true)]
    New {
        /// Repository path optionally including prefix (ex. gh:cpr-rs/cpp, cpr-rs/cpp)
        repo_path: String,
    },
    /// Set default git service
    #[command(arg_required_else_help = true)]
    Services {
        #[command(subcommand)]
        command: ServiceCommands,
    },
}

#[derive(Debug, Subcommand)]
enum ServiceCommands {
    /// Add a new service
    #[command(arg_required_else_help = true)]
    Add {
        /// Prefix for the service (ex. gh)
        prefix: String,
        /// URL format for the git server (ex. https://github.com/{{ repo }}.git)
        /// The `{{ repo }}` placeholder will be replaced with the repository name
        /// when creating a new project
        url: String,
    },
    /// Remove a service
    #[command(arg_required_else_help = true)]
    Remove {
        /// Prefix for the service (ex. gh)
        prefix: String,
    },
    /// List available services
    List,
    /// Set the default service
    /// The default service is used when a prefix is not specified
    #[command(arg_required_else_help = true)]
    Default {
        /// Prefix for the service (ex. gh)
        prefix: Option<String>,
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

    let mut config = Config::from_file(&config_path)?;

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
        Commands::Services { command } => match command {
            ServiceCommands::Add { prefix, url } => {
                config.add_service(prefix, url)?;
                config.write(&config_path)?;
            }
            ServiceCommands::Remove { prefix } => {
                config.remove_service(&prefix)?;
                config.write(&config_path)?;
            }
            ServiceCommands::List => {
                config.services.iter().for_each(|(prefix, base_url)| {
                    println!("`{}`: {}", prefix, base_url.url);
                });
            }
            ServiceCommands::Default { prefix } => {
                if let Some(prefix) = prefix {
                    config.set_default_service(&prefix)?;
                } else {
                    let service = requestty::Question::select("service")
                        .message("Select the default service")
                        .choices(config.services.keys().cloned())
                        .build();
                    let prefix = requestty::prompt_one(service).into_diagnostic()?;
                    config.set_default_service(&prefix.as_list_item().unwrap().text)?;
                }
                config.write(&config_path)?;
            }
        },
    }

    Ok(())
}
