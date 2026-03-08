// build.rs
// Wird von Cargo vor dem Kompilieren ausgeführt.
// Baut das Frontend-Bundle via webpack, sodass `cargo build`
// alles in einem Schritt erledigt.
//
// Projektstruktur (flat, kein frontend/-Unterordner):
//   package.json        ← im Root
//   webpack.config.js   ← im Root
//   static/main.js      ← Webpack-Einstiegspunkt
//   static/bundle.js    ← Webpack-Output
//   static/bundle.css   ← Webpack-Output

use std::process::Command;
use std::path::Path;

fn main() {
    // Cargo anweisen, bei Änderungen neu zu bauen.
    println!("cargo:rerun-if-changed=static/main.js");
    println!("cargo:rerun-if-changed=static/styles.css");
    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=webpack.config.js");

    // Arbeitsverzeichnis = Projekt-Root (wo Cargo.toml liegt)
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = Path::new(&root);

    // npm install – nur wenn node_modules fehlt
    if !root.join("node_modules").exists() {
        let status = Command::new("npm")
            .args(["install"])
            .current_dir(root)
            .status()
            .expect("npm install fehlgeschlagen – ist Node.js installiert und im PATH?");
        assert!(status.success(), "npm install returned non-zero exit code");
    }

    // npm run build → webpack → static/bundle.js + static/bundle.css
    let status = Command::new("npm")
        .args(["run", "build"])
        .current_dir(root)
        .status()
        .expect("npm run build fehlgeschlagen");
    assert!(status.success(), "webpack build returned non-zero exit code");
}