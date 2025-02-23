mod errors;
mod format;

use crate::errors::ProjectInitError;
use chrono::Datelike;
use clap::{Parser, Subcommand};
use errors::CPRConfigError;
use git2::{
    build::{CheckoutBuilder, RepoBuilder},
    FetchOptions, RemoteCallbacks,
};
use indicatif::{ProgressBar, ProgressStyle};
use miette::IntoDiagnostic;
use requestty::Question;
use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
};

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

fn prompt_project_info() -> miette::Result<ProjectInfo> {
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
    let answers = requestty::prompt(questions).into_diagnostic()?;
    let project_name = answers["project_name"].as_string().unwrap().to_string();
    let author = answers["author"].as_string().unwrap().to_string();

    Ok(ProjectInfo {
        project_name,
        author,
    })
}

fn prompt_template_questions(template_questions: Vec<toml::Value>) -> miette::Result<upon::Value> {
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
                                questions.push(match ty {
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
                                });
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
    for (key, answer) in requestty::prompt(questions).into_diagnostic()? {
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

    upon::to_value(map).into_diagnostic()
}

struct GitCloneState {
    total: usize,
    current: usize,
    path: Option<PathBuf>,
    started_resolution: bool,
}

fn clone_repository(directory: &Path, repo_path: String) -> miette::Result<()> {
    let clone_state = RefCell::new(GitCloneState {
        total: 0,
        current: 0,
        path: None,
        started_resolution: false,
    });
    let bar = ProgressBar::new(100);
    bar.set_style(
        ProgressStyle::with_template(
            "{spinner} [{elapsed_precise}] [{bar:30}] {pos:>7}/{len:7} {msg}",
        )
        .unwrap()
        .progress_chars("#>-"),
    );

    let mut cb = RemoteCallbacks::new();
    cb.transfer_progress(|stats| {
        let mut state = clone_state.borrow_mut();
        state.total = stats.total_objects();
        state.current = stats.received_objects();
        if state.current == state.total {
            // resolving deltas
            bar.set_message(format!(
                "Resolving deltas {}/{}",
                stats.indexed_deltas(),
                stats.total_deltas()
            ));
            if state.started_resolution {
                bar.reset();
            }
            bar.set_length(stats.total_deltas() as u64);
            bar.set_position(stats.indexed_deltas() as u64);
            state.started_resolution = true;
        } else {
            bar.set_length(state.total as u64);
            bar.set_position(state.current as u64);
            bar.set_message(format!(
                "Cloning {} ({}/{} objects)",
                repo_path, state.current, state.total
            ));
            bar.tick();
        }
        true
    });

    let mut co = CheckoutBuilder::new();
    co.progress(|path, cur, total| {
        let mut state = clone_state.borrow_mut();
        state.path = path.map(|p| p.to_path_buf());
        state.current = cur;
        state.total = total;
        if cur < total {
            bar.set_length(total as u64);
            bar.set_position(cur as u64);
            bar.set_message(format!(
                "Cloning {} ({}/{} objects)",
                repo_path, state.current, state.total
            ));
            bar.tick();
        }
    });

    let mut fo = FetchOptions::new();
    fo.remote_callbacks(cb);
    RepoBuilder::new()
        .fetch_options(fo)
        .with_checkout(co)
        .clone(&format!("https://github.com/{}.git", repo_path), directory)
        .map_err(|e| {
            if e.code() == git2::ErrorCode::NotFound {
                ProjectInitError::GitRepoNotFound
            } else {
                ProjectInitError::GitCloneFail
            }
        })
        .into_diagnostic()?;

    // let users decide their own vcs configuration
    std::fs::remove_dir_all(directory.join(".git"))
        .map_err(|_| ProjectInitError::GitCloneFail)
        .into_diagnostic()?;

    Ok(())
}

fn init(directory: PathBuf, repo_path: String, info: ProjectInfo) -> miette::Result<()> {
    clone_repository(&directory, repo_path)?;

    // if `cpr.toml` exists, use it
    let cpr_path = directory.join("cpr.toml");
    let mut template_answers = upon::Value::None;
    if cpr_path.exists() {
        let cpr = std::fs::read_to_string(&cpr_path)
            .map_err(|_| CPRConfigError::FileReadFail)
            .into_diagnostic()?;
        let cpr: toml::Value = toml::de::from_str(&cpr)
            .map_err(|_| CPRConfigError::TomlParseFail)
            .into_diagnostic()?;

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

    // skip read errors if enabled
    let mut skip_all = false;

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_dir() {
            continue;
        }

        let path = entry.into_path();

        let contents = std::fs::read_to_string(&path).map_err(|_| {
            ProjectInitError::ReadFileFail(path.file_name().unwrap().to_str().unwrap().to_string())
        });
        // instead of returning an error, we can prompt the user to skip the file
        // the options should be:
        // 1. Skip this error
        // 2. Skip all errors
        // 3. Abort

        if let Err(e) = contents {
            if skip_all {
                continue;
            }

            let opt = Question::select("err_policy")
                .message(format!(
                    "Failed to read file `{}`: What would you like to do?",
                    path.file_name().unwrap().to_str().unwrap(),
                ))
                .choices(vec![
                    "Skip this error".to_string(),
                    "Skip all future errors".into(),
                    "Abort".into(),
                ])
                .build();
            match requestty::prompt(vec![opt]).into_diagnostic()?["err_policy"]
                .as_list_item()
                .unwrap()
                .text
                .as_str()
            {
                "Skip this error" => continue,
                "Skip all future errors" => {
                    skip_all = true;
                    continue;
                }
                "Abort" => return Err(e).into_diagnostic(),
                _ => unreachable!(),
            }
        }

        let contents = contents.unwrap();

        engine.add_template("tmp", contents).into_diagnostic()?;

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
            .to_string()
            .into_diagnostic()?;

        std::fs::write(&path, result)
            .map_err(|_| {
                ProjectInitError::WriteFileFail(
                    path.file_name().unwrap().to_str().unwrap().to_string(),
                )
            })
            .into_diagnostic()?;

        engine.remove_template("tmp");
    }

    // remove cpr.toml
    if cpr_path.exists() && std::fs::remove_file(cpr_path).is_err() {
        eprintln!("! WARN: Failed to remove cpr.toml");
    }

    println!("Project initialized successfully");

    Ok(())
}

fn new(repo_path: String, info: ProjectInfo) -> miette::Result<()> {
    let project_dir = PathBuf::from(info.project_name.to_lowercase());

    if project_dir.exists() {
        println!("Project directory already exists");
        return Err(ProjectInitError::ProjectDirExists).into_diagnostic();
    }

    std::fs::create_dir(&project_dir)
        .map_err(|_| ProjectInitError::ProjectDirCreateFail)
        .into_diagnostic()?;

    init(project_dir, repo_path, info)?;

    Ok(())
}

fn main() -> miette::Result<()> {
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
