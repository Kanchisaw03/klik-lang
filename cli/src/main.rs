// KLIK CLI - Main entry point

use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;

mod commands;
#[path = "../../compiler/pipeline.rs"]
mod pipeline;

#[derive(Parser)]
#[command(
    name = "klik",
    about = "The KLIK programming language toolkit",
    version,
    author
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a KLIK project in the current directory or with a name
    Init {
        /// Project name (defaults to current directory name)
        name: Option<String>,
        /// Directory to create project in (defaults to current dir)
        #[arg(long)]
        path: Option<PathBuf>,
    },

    /// Create a new KLIK project
    New {
        /// Project name
        name: String,
        /// Directory to create project in (defaults to current dir)
        #[arg(long)]
        path: Option<PathBuf>,
    },

    /// Build the current project
    Build {
        /// Input .klik file (if omitted, builds current project entry file)
        input: Option<PathBuf>,
        /// Build in release mode with optimizations
        #[arg(long)]
        release: bool,
        /// Target platform
        #[arg(long, default_value = "native")]
        target: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Preserve intermediate artifacts (.ir and .obj)
        #[arg(long)]
        emit_all: bool,
        /// Optimization level: O0, O1, O2
        #[arg(long, default_value = "O1")]
        opt_level: String,
        /// Emit textual IR and IR DOT graph for debugging
        #[arg(long)]
        emit_ir: bool,
        /// Emit AST DOT graph for debugging
        #[arg(long)]
        emit_ast: bool,
        /// Emit CFG DOT graph for debugging
        #[arg(long)]
        emit_cfg: bool,
    },

    /// Generate AST/IR/CFG visualizations
    Visualize {
        /// Input .klik file
        input: PathBuf,
        /// Automatically open visualization/pipeline.html
        #[arg(long)]
        open: bool,
    },

    /// Validate native vs Cranelift backend outputs across core examples
    TestBackend,

    /// Run the current project
    Run {
        /// Input .klik file (if omitted, runs current project entry file)
        input: Option<PathBuf>,
        /// Build in release mode
        #[arg(long)]
        release: bool,
        /// Enable compiler pipeline tracing
        #[arg(long)]
        trace: bool,
        /// Arguments to pass to the program
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Check the project for errors without building
    Check {
        /// Show all warnings
        #[arg(long)]
        all_warnings: bool,
    },

    /// Run tests
    Test {
        /// Filter tests by name
        #[arg(long)]
        filter: Option<String>,
        /// Show output from passing tests
        #[arg(long)]
        show_output: bool,
    },

    /// Format source files
    Fmt {
        /// Check formatting without modifying files
        #[arg(long)]
        check: bool,
        /// Specific files to format
        files: Vec<PathBuf>,
    },

    /// Run the linter
    Lint {
        /// Specific files to lint
        files: Vec<PathBuf>,
        /// Fix auto-fixable issues
        #[arg(long)]
        fix: bool,
    },

    /// Add a dependency
    Add {
        /// Package name
        name: String,
        /// Version requirement
        #[arg(long, default_value = "*")]
        version: String,
        /// Add as dev dependency
        #[arg(long)]
        dev: bool,
    },

    /// Remove a dependency
    Remove {
        /// Package name
        name: String,
    },

    /// Start the language server
    Lsp,

    /// Run with hot reload (watches for changes)
    Watch {
        /// Arguments to pass to the program
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Clean build artifacts
    Clean,

    /// Show project info
    Info,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { name, path } => commands::init_project(name.as_deref(), path),
        Commands::New { name, path } => commands::new_project(&name, path),
        Commands::Build {
            input,
            release,
            target,
            output,
            emit_all,
            opt_level,
            emit_ir,
            emit_ast,
            emit_cfg,
        } => commands::build_with_options(
            input.as_deref(),
            release,
            &target,
            output,
            emit_all,
            &opt_level,
            emit_ir,
            emit_ast,
            emit_cfg,
            false,
        ),
        Commands::Visualize { input, open } => commands::visualize(&input, open),
        Commands::Run {
            input,
            release,
            trace,
            args,
        } => commands::run(input.as_deref(), release, trace, &args),
        Commands::Check { all_warnings } => commands::check(all_warnings),
        Commands::Test {
            filter,
            show_output,
        } => commands::test(filter.as_deref(), show_output),
        Commands::Fmt { check, files } => commands::fmt(check, &files),
        Commands::Lint { files, fix } => commands::lint(&files, fix),
        Commands::Add { name, version, dev } => commands::add_dep(&name, &version, dev),
        Commands::Remove { name } => commands::remove_dep(&name),
        Commands::Lsp => commands::start_lsp(),
        Commands::Watch { args } => commands::watch(&args),
        Commands::Clean => commands::clean(),
        Commands::Info => commands::info(),
        Commands::TestBackend => commands::test_backend(),
    };

    if let Err(e) = result {
        eprintln!("{} {:#}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}
