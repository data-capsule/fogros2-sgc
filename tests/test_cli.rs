#[cfg(test)]
extern crate assert_cmd;
extern crate predicates;
use assert_cmd::prelude::*;

use std::process::Command;

#[test]
fn test_cli() {
    let mut cmd = Command::cargo_bin("gdp-router").expect("Calling binary failed");
    cmd.assert().failure();
}

#[test]
fn test_version() {
    let expected_version = "gdp-router 0.0.1-beta\n";
    let mut cmd = Command::cargo_bin("gdp-router").expect("Calling binary failed");
    cmd.arg("--version").assert().stdout(expected_version);
}
