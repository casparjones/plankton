// build.rs
// Baut das Frontend via webpack – wird von Cargo vor dem Kompilieren ausgeführt.
//
// Logik:
// - Wenn static/bundle.js bereits existiert (z.B. im Docker-Build von Stage 1
//   vorbereitet), wird npm übersprungen → kein Node.js im Rust-Builder nötig
// - Wenn bundle.js fehlt (lokale Entwicklung), wird npm install + build ausgeführt

use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=static/main.js");
    println!("cargo:rerun-if-changed=static/styles.css");
    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=webpack.config.js");

    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = Path::new(&root);
    let bundle = root.join("static/bundle.js");

    // Bundle bereits vorhanden (Docker Stage 1 oder manuelles npm run build)?
    // Dann überspringen – kein Node.js erforderlich.
    if bundle.exists() {
        println!("cargo:warning=bundle.js gefunden – npm build übersprungen");
        return;
    }

    // Lokale Entwicklung: node_modules installieren falls nötig
    if !root.join("node_modules").exists() {
        let status = Command::new("npm")
            .args(["install"])
            .current_dir(root)
            .status()
            .expect("npm install fehlgeschlagen – ist Node.js installiert?");
        assert!(status.success(), "npm install schlug fehl");
    }

    // Webpack-Bundle bauen
    let status = Command::new("npm")
        .args(["run", "build"])
        .current_dir(root)
        .status()
        .expect("npm run build fehlgeschlagen");
    assert!(status.success(), "webpack build schlug fehl");
}