use std::process::Command;

use ts_rs::TS;

fn main() {
    println!("cargo:rerun-if-changed=../shared/src");
    println!("cargo:rerun-if-changed=../frontend/src");
    println!("cargo:rerun-if-changed=../frontend/index.html");
    println!("cargo:rerun-if-changed=../frontend/package.json");

    let cfg = ts_rs::Config::new();
    shared::ClientMessage::export_all(&cfg).unwrap();
    shared::ServerMessage::export_all(&cfg).unwrap();
    shared::Game::export_all(&cfg).unwrap();

    let frontend = concat!(env!("CARGO_MANIFEST_DIR"), "/../frontend");
    if !std::path::Path::new(&format!("{frontend}/node_modules")).exists() {
        run("npm", &["install"], frontend);
    }
    run("npm", &["run", "build"], frontend);
}

fn run(cmd: &str, args: &[&str], dir: &str) {
    let status = Command::new(cmd)
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn {cmd}: {e}"));

    assert!(status.success(), "{cmd} {args:?} failed in {dir}");
}
