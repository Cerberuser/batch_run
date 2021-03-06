use crate::result::{error::BatchError, BatchResult};
use crate::result::{error::EntryError, EntryResult};
use lazy_static::lazy_static;
use std::{
    env::{consts::EXE_EXTENSION, var_os},
    path::{Path, PathBuf},
    process::{Command, Output},
};

use crate::binary::BinaryBuilder;
use crate::rustflags;

lazy_static! {
    static ref TARGET_BIN: PathBuf = {
        let mut tmp: PathBuf = [".", "target", "batch"].iter().collect();
        // TODO configurable?
        std::fs::create_dir_all(&tmp).expect("Unable to create batch executable in target directory; check your access rights");
        tmp.push(&format!("{:x}", rand::random::<u64>()));
        tmp.with_extension(EXE_EXTENSION)
    };
}

include!(concat!(env!("OUT_DIR"), "/info.rs"));

fn raw_cargo() -> Command {
    Command::new(option_env!("CARGO").unwrap_or("cargo"))
}

fn rustc() -> Command {
    let mut cmd = Command::new(info::rustc());
    cmd.current_dir(var_os("CARGO_MANIFEST_DIR").unwrap());
    cmd.args(&[
        "-o",
        TARGET_BIN.to_str().expect("Non-UTF-8 symbols in path"),
    ]);
    cmd
}

pub fn capture_build_command(bin_name: &str) -> BatchResult<String> {
    let mut cmd = raw_cargo();
    cmd.current_dir(var_os("CARGO_MANIFEST_DIR").unwrap());
    rustflags::set_env(&mut cmd);
    cmd.arg("build");
    if info::opt_level() == "release" {
        cmd.arg("--release");
    };
    cmd.arg("--bin")
        .arg(bin_name)
        .arg("--verbose")
        .output()
        .map_err(BatchError::Cargo)
        .map_err(Into::into)
        // .map(|out| { println!("Cargo output: \"{}\"", String::from_utf8(out.clone().stderr).unwrap()); out })
        .map(extract_build_command)
        .map(trim_build_command)
}

fn extract_build_command(out: Output) -> String {
    String::from_utf8(out.stderr)
        .expect("Cargo produced non-UTF-8 output")
        .lines()
        // .inspect(|line| println!("Cargo output: {}", line))
        .filter(|line| line.trim_start().starts_with("Running `"))
        .last()
        .expect("No running command in cargo output")
        .to_owned()
}

fn trim_build_command(line: String) -> String {
    line.trim_start_matches("Running")
        .trim()
        .trim_matches('`')
        .to_owned()
}

pub fn build_entry(builder: &BinaryBuilder, main: &Path, run: bool) -> EntryResult<Output> {
    let mut cmd = rustc();
    builder.args_to_command(&mut cmd, main);
    cmd.arg(if run {
        "--emit=link"
    } else {
        "--emit=dep-info"
    });
    cmd.output().map_err(EntryError::Rustc).map_err(Into::into)
}

pub fn run_entry() -> EntryResult<Output> {
    Command::new(&*TARGET_BIN)
        .output()
        .map_err(EntryError::RunFailed)
        .map_err(Into::into)
}
