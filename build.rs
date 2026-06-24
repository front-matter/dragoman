fn main() {
    // Rebuild whenever Zola source files change.
    println!("cargo:rerun-if-changed=site/config.toml");
    println!("cargo:rerun-if-changed=site/templates/");
    println!("cargo:rerun-if-changed=site/content/");
    println!("cargo:rerun-if-changed=site/static/");

    // Run `zola build` if Zola is installed. If not, the committed
    // site/public/index.html is used as-is (CI does not need Zola).
    let ok = std::process::Command::new("zola")
        .args(["build"])
        .current_dir("site")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !ok && !std::path::Path::new("site/public/index.html").exists() {
        panic!("site/public/index.html is missing; install zola and run `zola build` inside site/");
    }
}
