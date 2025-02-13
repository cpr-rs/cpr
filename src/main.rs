mod errors;
mod format;

use crate::errors::ProjectInitError;
use chrono::Datelike;
use clap::{Parser, Subcommand};
use requestty::Question;
use std::{collections::HashMap, path::PathBuf};

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
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Initialize a directory with a template from GitHub
    #[command(arg_required_else_help = true)]
    Init {
        /// Directory to target (ex. ./my_project)
        directory: PathBuf,
        /// Repository path from GitHub (ex. cpr-rs/cpp)
        repo_path: String,
    },
    /// Create a new project with a template from GitHub
    #[command(arg_required_else_help = true)]
    New {
        /// Repository path from GitHub (ex. cpr-rs/cpp)
        repo_path: String,
    },
}

struct ProjectInfo {
    project_name: String,
    author: String,
}

fn prompt_project_info() -> anyhow::Result<ProjectInfo> {
    let questions = vec![
        Question::input("project_name")
            .message("Project name?")
            .default("my_project")
            .build(),
        Question::input("author")
            .message("Author?")
            .default("John Doe")
            .build(),
    ];
    let answers = requestty::prompt(questions)?;
    let project_name = answers["project_name"].as_string().unwrap().to_string();
    let author = answers["author"].as_string().unwrap().to_string();

    Ok(ProjectInfo {
        project_name,
        author,
    })
}

fn prompt_template_questions(template_questions: Vec<toml::Value>) -> anyhow::Result<upon::Value> {
    let mut questions = Vec::<Question>::with_capacity(template_questions.len());
    for template_question in template_questions {
        match template_question {
            toml::Value::Table(table) => {
                let key = table.get("key").unwrap().as_str().unwrap();
                let message = table.get("message").unwrap().as_str().unwrap();
                let ty = table.get("type").unwrap().as_str().unwrap();

                match ty {
                    "confirm" => {
                        questions.push(Question::confirm(key).message(message).build());
                    }
                    "input" => {
                        questions.push(Question::input(key).message(message).build());
                    }
                    "int" => {
                        questions.push(Question::int(key).message(message).build());
                    }
                    "float" => {
                        questions.push(Question::float(key).message(message).build());
                    }
                    "select" | "multi_select" | "order_select" => {
                        let choices = table.get("choices").unwrap();
                        match choices {
                            toml::Value::Array(choices) => {
                                let mut items =
                                    Vec::<requestty::question::Choice<String>>::with_capacity(
                                        choices.len(),
                                    );
                                let mut items_raw = Vec::<String>::with_capacity(choices.len());
                                for choice in choices {
                                    if let toml::Value::String(choice) = choice {
                                        if choice == "cpr_sep" {
                                            items.push(
                                                requestty::question::Choice::DefaultSeparator,
                                            );
                                        } else {
                                            items.push(choice.into());
                                            items_raw.push(choice.clone());
                                        }
                                    }
                                }
                                questions.push(
                                    match ty {
                                        "select" => Question::select(key)
                                            .message(message)
                                            .choices(items)
                                            .build(),
                                        "multi_select" => Question::multi_select(key)
                                            .message(message)
                                            .choices(items)
                                            .build(),
                                        "order_select" => Question::order_select(key)
                                            .message(message)
                                            .choices(items_raw)
                                            .build(),
                                        _ => unreachable!(),
                                    },
                                );
                            }
                            _ => {
                                eprintln!(
                                    "! WARN: Expected array of *tables* for [questions.{}.choices] in cpr.toml, skipping",
                                    key
                                );
                            }
                        }
                    }
                    _ => {
                        eprintln!(
                            "! WARN: Unknown question type `{}` for key `{}`, skipping",
                            ty, key
                        );
                    }
                }
            }
            _ => {
                eprintln!(
                    "! WARN: Expected array of *tables* for [questions] in cpr.toml, skipping"
                );
            }
        }
    }

    let mut map = HashMap::<String, upon::Value>::new();
    for (key, answer) in requestty::prompt(questions)? {
        use requestty::{Answer, ExpandItem, ListItem};
        map.insert(
            key,
            match answer {
                Answer::String(str) => upon::Value::String(str),
                Answer::ListItem(ListItem { text, .. }) => upon::Value::String(text),
                Answer::ExpandItem(ExpandItem { text, .. }) => upon::Value::String(text),
                Answer::Int(num) => upon::Value::Integer(num),
                Answer::Float(num) => upon::Value::Float(num),
                Answer::Bool(bool) => upon::Value::Bool(bool),
                Answer::ListItems(mut items) => {
                    items.sort_by(|a, b| a.index.cmp(&b.index));
                    upon::Value::List(
                        items
                            .into_iter()
                            .map(|item| upon::Value::String(item.text))
                            .collect(),
                    )
                }
            },
        );
    }

    Ok(upon::to_value(map)?)
}

fn init(directory: PathBuf, repo_path: String, info: ProjectInfo) -> anyhow::Result<()> {
    let _ = git2::Repository::clone(&format!("https://github.com/{}.git", repo_path), &directory)
        .map_err(|_| ProjectInitError::GitCloneFailed)?;

    // let users decide their own vcs configuration
    std::fs::remove_dir_all(directory.join(".git"))
        .map_err(|_| ProjectInitError::GitCloneFailed)?;

    // if `cpr.toml` exists, use it
    let cpr_path = directory.join("cpr.toml");
    let mut template_answers = upon::Value::None;
    if cpr_path.exists() {
        let cpr = std::fs::read_to_string(&cpr_path)
            .map_err(|_| ProjectInitError::ProjectDirCreateFailed)?;
        let cpr: toml::Value =
            toml::de::from_str(&cpr).map_err(|_| ProjectInitError::ProjectDirCreateFailed)?;

        template_answers = match cpr {
            toml::Value::Table(table) => {
                if let Some(qs) = table.get("questions") {
                    match qs {
                        toml::Value::Array(questions) => {
                            prompt_template_questions(questions.clone())?
                        }
                        _ => {
                            eprintln!("! WARN: Expected *array* of tables for [questions] in cpr.toml, skipping");
                            upon::Value::None
                        }
                    }
                } else {
                    eprintln!("! WARN: No questions found in cpr.toml");
                    upon::Value::None
                }
            }
            _ => upon::Value::None,
        };
    }

    let year = chrono::offset::Local::now().year();

    // walk the directory and run template engine
    let mut engine = upon::Engine::new();
    let walker = walkdir::WalkDir::new(&directory).sort_by_file_name();

    engine.add_formatter("lower", format::lower);
    engine.add_formatter("upper", format::upper);
    engine.add_formatter("snake", format::snake);
    engine.add_formatter("kebab", format::kebab);
    engine.add_formatter("pascal", format::pascal);
    engine.add_formatter("camel", format::camel);
    engine.add_formatter("title", format::title);

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_dir() {
            continue;
        }

        let path = entry.into_path();
        let contents =
            std::fs::read_to_string(&path).map_err(|_| ProjectInitError::ProjectDirCreateFailed)?;
        engine.add_template("tmp", contents)?;

        let result = engine
            .template("tmp")
            // includes defaults for: {{ project.name }} {{ year }} {{ author }}
            .render(upon::value! {
                project: {
                    name: &info.project_name,
                },
                year: year,
                author: &info.author,
                cpr: &template_answers,
            })
            .to_string()?;

        std::fs::write(path, result).map_err(|_| ProjectInitError::ProjectDirCreateFailed)?;

        engine.remove_template("tmp");
    }

    // remove cpr.toml
    std::fs::remove_file(cpr_path).map_err(|_| ProjectInitError::ProjectDirCreateFailed)?;

    println!("Project initialized successfully");

    Ok(())
}

fn new(repo_path: String, info: ProjectInfo) -> anyhow::Result<()> {
    let project_dir = PathBuf::from(info.project_name.to_lowercase());

    if project_dir.exists() {
        println!("Project directory already exists");
        return Err(ProjectInitError::ProjectDirExists.into());
    }

    std::fs::create_dir(&project_dir).map_err(|_| ProjectInitError::ProjectDirCreateFailed)?;

    init(project_dir, repo_path, info)?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Init {
            directory,
            repo_path,
        } => {
            init(directory, repo_path, prompt_project_info()?)?;
        }
        Commands::New { repo_path } => {
            new(repo_path, prompt_project_info()?)?;
        }
    }

    Ok(())
}
