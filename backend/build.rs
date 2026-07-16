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
    let npm_works = Command::new("npm")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success() && !o.stdout.is_empty());
    if !npm_works {
        assert!(
            std::path::Path::new(&format!("{frontend}/dist")).exists(),
            "npm is not available and {frontend}/dist is missing; run `npm run build` in frontend/ first"
        );
        return;
    }
    if !std::path::Path::new(&format!("{frontend}/node_modules")).exists() {
        run("npm", &["install"], frontend);
    }
    run("npm", &["run", "build"], frontend);
}

fn run(cmd: &str, args: &[&str], dir: &str) {
    let output = Command::new(cmd)
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {cmd}: {e}"));

    assert!(
        output.status.success(),
        "{cmd} {args:?} failed in {dir} with {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
