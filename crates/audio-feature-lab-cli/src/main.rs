use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use audio_feature_lab_config::LabConfig;

fn main() -> ExitCode {
    run()
}

fn run() -> ExitCode {
    let mut args = env::args_os();
    let program = args
        .next()
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| "audio-feature-lab".to_string());

    let Some(command) = args.next() else {
        print_help(&program);
        return ExitCode::SUCCESS;
    };

    match command.to_str() {
        Some("-h") | Some("--help") => {
            print_help(&program);
            ExitCode::SUCCESS
        }
        Some("-V") | Some("--version") => {
            println!("{program} {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("validate-config") => validate_config(args),
        Some("analyze") | Some("batch") | Some("scan") => reserved_command(command),
        Some(other) => {
            eprintln!("unknown command: {other}");
            print_help(&program);
            ExitCode::from(2)
        }
        None => {
            eprintln!("command names must be valid UTF-8 in this scaffold");
            ExitCode::from(2)
        }
    }
}

fn validate_config(mut args: impl Iterator<Item = OsString>) -> ExitCode {
    let path = args.next();

    if args.next().is_some() {
        eprintln!("usage: audio-feature-lab validate-config [path]");
        return ExitCode::from(2);
    }

    let path = path.as_ref().map(PathBuf::from);
    match LabConfig::load(path.as_deref()) {
        Ok(config) => {
            println!("config is valid: profile={}", config.profile.as_str());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("invalid config: {error}");
            ExitCode::from(1)
        }
    }
}

fn reserved_command(command: OsString) -> ExitCode {
    let name = command.to_string_lossy();
    eprintln!("`{name}` is reserved for a later execution-plan phase");
    ExitCode::from(2)
}

fn print_help(program: &str) {
    println!("Usage: {program} <command> [args]");
    println!();
    println!("Commands:");
    println!("  analyze          Reserved for a later phase");
    println!("  batch            Reserved for a later phase");
    println!("  scan             Reserved for a later phase");
    println!("  validate-config  Validate a config file or the default profile");
}
