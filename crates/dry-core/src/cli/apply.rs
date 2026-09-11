//! Flag application and config overlays for CLI parsing.

use std::path::PathBuf;

use crate::{Config, OutputFormat};

use super::{CliError, RawFlags, help_text};

pub(super) fn apply_arg(
    arg: &str,
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Result<(), CliError> {
    if let Some(result) = try_flag_arg(arg, args, raw) {
        return result;
    }
    raw.paths.push(PathBuf::from(arg));
    Ok(())
}

fn try_flag_arg(
    arg: &str,
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Option<Result<(), CliError>> {
    if arg == "--help" || arg == "-h" {
        return Some(Err(CliError::help(help_text(raw.bin_name))));
    }
    try_apply_valued(arg, args, raw)
        .or_else(|| try_apply_switch(arg, raw))
        .or_else(|| unknown_flag(arg))
}

fn unknown_flag(arg: &str) -> Option<Result<(), CliError>> {
    if arg.starts_with('-') {
        Some(Err(CliError::usage(format!("unknown flag: {arg}"))))
    } else {
        None
    }
}

fn try_apply_valued(
    arg: &str,
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Option<Result<(), CliError>> {
    // dry-rs:ignore. CC-driven one-flag CLI helpers; parallel shape is intentional.
    try_apply_configish(arg, args, raw).or_else(|| try_apply_outputish(arg, args, raw))
}

fn try_apply_configish(
    arg: &str,
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Option<Result<(), CliError>> {
    // dry-rs:ignore. CC-driven one-flag CLI helpers; parallel shape is intentional.
    try_apply_config_threshold(arg, args, raw).or_else(|| try_apply_mins(arg, args, raw))
}

fn try_apply_config_threshold(
    arg: &str,
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Option<Result<(), CliError>> {
    // dry-rs:ignore. CC-driven one-flag CLI helpers; parallel shape is intentional.
    match arg {
        "--config" => Some(apply_config(args, raw)),
        "--threshold" => Some(apply_threshold(args, raw)),
        _ => None,
    }
}

fn try_apply_mins(
    arg: &str,
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Option<Result<(), CliError>> {
    // dry-rs:ignore. CC-driven one-flag CLI helpers; parallel shape is intentional.
    match arg {
        "--min-nodes" => Some(apply_min_nodes(args, raw)),
        "--min-lines" => Some(apply_min_lines(args, raw)),
        _ => None,
    }
}

fn try_apply_outputish(
    arg: &str,
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Option<Result<(), CliError>> {
    // dry-rs:ignore. CC-driven one-flag CLI helpers; parallel shape is intentional.
    match arg {
        "--format" => Some(apply_format(args, raw)),
        "--json-out" => Some(apply_json_out(args, raw)),
        _ => None,
    }
}

fn try_apply_switch(arg: &str, raw: &mut RawFlags) -> Option<Result<(), CliError>> {
    match arg {
        "--fail-on-findings" => {
            raw.fail_on = Some(true);
            Some(Ok(()))
        }
        "--no-fail-on-findings" => {
            raw.fail_on = Some(false);
            Some(Ok(()))
        }
        _ => None,
    }
}

fn apply_config(
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Result<(), CliError> {
    // dry-rs:ignore. CC-driven one-flag CLI helpers; parallel shape is intentional.
    raw.config_path = Some(PathBuf::from(require_value(args, "--config")?));
    Ok(())
}

fn apply_threshold(
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Result<(), CliError> {
    // dry-rs:ignore. CC-driven one-flag CLI helpers; parallel shape is intentional.
    raw.threshold = Some(parse_value(&require_value(args, "--threshold")?, "float")?);
    Ok(())
}

fn apply_format(
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Result<(), CliError> {
    let value = require_value(args, "--format")?;
    raw.format =
        Some(OutputFormat::parse(&value).ok_or_else(|| CliError::usage("invalid --format"))?);
    Ok(())
}

fn apply_min_nodes(
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Result<(), CliError> {
    // dry-rs:ignore. CC-driven one-flag CLI helpers; parallel shape is intentional.
    raw.min_nodes = Some(parse_value(
        &require_value(args, "--min-nodes")?,
        "integer",
    )?);
    Ok(())
}

fn apply_min_lines(
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Result<(), CliError> {
    // dry-rs:ignore. CC-driven one-flag CLI helpers; parallel shape is intentional.
    raw.min_lines = Some(parse_value(
        &require_value(args, "--min-lines")?,
        "integer",
    )?);
    Ok(())
}

fn apply_json_out(
    args: &mut impl Iterator<Item = String>,
    raw: &mut RawFlags,
) -> Result<(), CliError> {
    // dry-rs:ignore. CC-driven one-flag CLI helpers; parallel shape is intentional.
    raw.json_out = Some(PathBuf::from(require_value(args, "--json-out")?));
    Ok(())
}

pub(super) const fn overlay_cli_onto_config(raw: &RawFlags, config: &mut Config) {
    overlay_gate_flags(raw, config);
    overlay_output_flags(raw, config);
    overlay_walk_flags(raw, config);
}

const fn overlay_gate_flags(raw: &RawFlags, config: &mut Config) {
    if let Some(threshold) = raw.threshold {
        config.gate.threshold = threshold;
    }
    if let Some(fail_on) = raw.fail_on {
        config.gate.fail_on_findings = fail_on;
    }
}

const fn overlay_output_flags(raw: &RawFlags, config: &mut Config) {
    if let Some(format) = raw.format {
        config.output.format = format;
    }
}

const fn overlay_walk_flags(raw: &RawFlags, config: &mut Config) {
    if let Some(min_nodes) = raw.min_nodes {
        config.walk.min_nodes = min_nodes;
    }
    if let Some(min_lines) = raw.min_lines {
        config.walk.min_lines = min_lines;
    }
}

fn require_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, CliError> {
    args.next().ok_or_else(|| CliError::usage(format!("missing value for {flag}")))
}

fn parse_value<T: std::str::FromStr>(raw: &str, kind: &str) -> Result<T, CliError> {
    raw.parse::<T>().map_err(|_| CliError::usage(format!("invalid {kind}: {raw}")))
}
