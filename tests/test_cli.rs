#[cfg(test)]
extern crate assert_cmd;
extern crate predicates;

use assert_cmd::prelude::*;
use predicates::prelude::*;

use std::process::Command;

#[test]
fn test_cli() {
    let mut cmd = Command::cargo_bin("rust-starter").expect("Calling binary failed");
    cmd.assert().failure();
}

#[test]
fn test_version() {
    let expected_version = "gdp-router 2.0.0-beta\n";
    let mut cmd = Command::cargo_bin("rust-starter").expect("Calling binary failed");
    cmd.arg("--version").assert().stdout(expected_version);
}
