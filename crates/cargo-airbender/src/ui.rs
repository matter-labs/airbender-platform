use crate::error::CliError;
use anstyle::{AnsiColor, Effects, Style};
use std::fmt::Display;
use std::io::Write;

pub fn success(message: impl AsRef<str>) {
    if !should_emit() {
        return;
    }
    print_stdout_line("success", AnsiColor::Green, message.as_ref());
}

pub fn info(message: impl AsRef<str>) {
    if !should_emit() {
        return;
    }
    print_stdout_line("info", AnsiColor::Blue, message.as_ref());
}

pub fn field(key: &str, value: impl Display) {
    if !should_emit() {
        return;
    }
    let mut stream = anstream::stdout();
    let _ = writeln!(stream, "  {key}: {value}");
}

pub fn command(command: impl AsRef<str>) {
    if !should_emit() {
        return;
    }
    let style = label_style(AnsiColor::Cyan);
    let mut stream = anstream::stdout();
    let _ = writeln!(
        stream,
        "  {style}$ {command}{style:#}",
        command = command.as_ref()
    );
}

pub fn blank_line() {
    if !should_emit() {
        return;
    }
    let mut stream = anstream::stdout();
    let _ = writeln!(stream);
}

pub fn render_error(err: &CliError) {
    if !should_emit() {
        return;
    }
    print_stderr_line("error", AnsiColor::Red, err.to_string());

    if let Some(source) = err.source_error() {
        if let Some(root_cause) = source.chain().last() {
            print_stderr_detail("cause", root_cause);
        }
    }

    if let Some(hint) = err.hint() {
        print_stderr_line("hint", AnsiColor::Yellow, hint);
    }
}

fn print_stdout_line(label: &str, color: AnsiColor, message: &str) {
    let style = label_style(color);
    let mut stream = anstream::stdout();
    let _ = writeln!(stream, "{style}{label}{style:#}: {message}");
}

fn print_stderr_line(label: &str, color: AnsiColor, message: impl Display) {
    let style = label_style(color);
    let mut stream = anstream::stderr();
    let _ = writeln!(stream, "{style}{label}{style:#}: {message}");
}

fn print_stderr_detail(label: &str, message: impl Display) {
    let mut stream = anstream::stderr();
    let _ = writeln!(stream, "  {label}: {message}");
}

fn label_style(color: AnsiColor) -> Style {
    Style::new()
        .fg_color(Some(color.into()))
        .effects(Effects::BOLD)
}

fn should_emit() -> bool {
    !cfg!(test)
}
