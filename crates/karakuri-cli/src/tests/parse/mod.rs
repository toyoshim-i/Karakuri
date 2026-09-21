use super::*;
use karakuri_environment::compile::sort_compiled;
use karakuri_signal::NoiseKind;

pub(crate) fn parse(args: &[&str]) -> Result<Args, String> {
    match parse_args_from(args.iter().map(|s| s.to_string())) {
        Ok(ParseOutcome::Run(args)) => Ok(*args),
        Ok(ParseOutcome::Help) => panic!("expected Args, got --help"),
        Ok(ParseOutcome::ListSets(_)) => panic!("expected Args, got --list-sets"),
        Ok(ParseOutcome::Package { .. }) => panic!("expected Args, got --package"),
        Ok(ParseOutcome::TakeIn { .. }) => panic!("expected Args, got --take-in"),
        Err(e) => Err(e),
    }
}

mod binding;
mod devices;
mod library_ops;
mod options;
mod session_head;
