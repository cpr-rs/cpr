# CPR

## Description

CPR is a simple command line utility for managing C/C++ projects through templates. It allows you to initialize a new project and start working quickly while also giving template maintainers flexibility in how they want to structure their templates.

## Features

- Fetch templates from GitHub
- Custom template questions
- `if` and `for` statements in templates
- Nested templates (TODO)

## Non-Goals

While the structure of this project is not fully defined yet, the following are **NOT** goals of this project:

- A full-fledged build system
- A package manager

## Installation

### Prerequisites

- Cargo
- Rust 1.74.1 or later

### Using `cargo install`

```bash
cargo install cpr-cli
```

### Using `cargo build`

```bash
git clone https://github.com/cpr-rs/cpr.git
cd cpr
cargo build --release
```

## Usage

```helptext
A simple git-based project manager aimed at C/C++

Usage: cpr <COMMAND>

Commands:
  init  Initialize a directory with a template from GitHub
  new   Create a new project with a template from GitHub
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
