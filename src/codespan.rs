use std::path::PathBuf;

use ariadne::{Label, ReportKind, Source};
use color_eyre::owo_colors::OwoColorize;
use sysexits::ExitCode;
use toml::de;

use crate::config::ConfigError;

pub fn report_toml_error(prefix: String, file: PathBuf, source: String, error: de::Error) -> ! {
    let Some(span) = error.span() else {
        eprintln!(
            "{label} {prefix} - {message}",
            label = "Error:".red(),
            message = error.message()
        );

        ExitCode::Config.exit();
    };

    let file = file.display().to_string();

    let report = ariadne::Report::build(ReportKind::Error, (&file, span.clone()))
        .with_message(format!("{prefix} - failed to parse toml"))
        .with_label(Label::new((&file, span)).with_message(error.message()))
        .finish();

    if report.eprint((&file, Source::from(source))).is_err() {
        eprintln!(
            "{label} {prefix} {message}",
            label = "Error:".red(),
            message = error.message()
        );
    }

    ExitCode::Config.exit();
}

pub fn report_config_error(file: PathBuf, source: String, error: ConfigError) -> ! {
    let report = error.report(file.clone());

    if report
        .eprint((file.display().to_string(), Source::from(source)))
        .is_err()
    {
        eprintln!(
            "{label} Invalid configuration - {error:?}",
            label = "Error:".red(),
        );
    }

    ExitCode::Config.exit();
}
