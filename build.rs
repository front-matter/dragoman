fn main() {
    // Rebuild whenever ui source files change.
    println!("cargo:rerun-if-changed=ui/src/");
    println!("cargo:rerun-if-changed=ui/index.html");
    println!("cargo:rerun-if-changed=ui/package.json");
    println!("cargo:rerun-if-changed=ui/vite.config.js");
    println!("cargo:rerun-if-changed=ui/svelte.config.js");

    // pnpm may live under Homebrew on macOS; prepend common prefix to PATH.
    let path = std::env::var("PATH").unwrap_or_default();
    let path = format!("/opt/homebrew/bin:/usr/local/bin:{path}");

    let ok = std::process::Command::new("pnpm")
        .args(["--dir", "ui", "build"])
        .env("PATH", &path)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !ok && !std::path::Path::new("ui/dist/index.html").exists() {
        panic!(
            "ui/dist/index.html is missing; \
             run: pnpm --dir ui install && pnpm --dir ui build"
        );
    }
}
