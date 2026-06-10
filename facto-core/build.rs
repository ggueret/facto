//! Build-time provenance: embed the git commit (short SHA, date, dirty flag)
//! the binary was compiled from, so `facto --version` and the MCP handshake
//! can report exactly which code is running. Falls back to "unknown" when
//! built outside a git checkout (e.g. from a crates.io tarball).

use std::path::Path;
use std::process::Command;

fn main() {
    let sha = git(&["rev-parse", "--short=7", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
    let date = git(&["show", "-s", "--format=%cs", "HEAD"]).unwrap_or_default();
    let dirty = git(&["status", "--porcelain"]).is_some();

    let pkg = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
    let dirty_suffix = if dirty { "-dirty" } else { "" };

    let long = if date.is_empty() {
        format!("{pkg} ({sha}{dirty_suffix})")
    } else {
        format!("{pkg} ({sha}{dirty_suffix} {date})")
    };
    let mcp = format!("{pkg}+{sha}{}", if dirty { ".dirty" } else { "" });

    println!("cargo:rustc-env=FACTO_GIT_SHA={sha}");
    println!("cargo:rustc-env=FACTO_GIT_DATE={date}");
    println!("cargo:rustc-env=FACTO_LONG_VERSION={long}");
    println!("cargo:rustc-env=FACTO_MCP_VERSION={mcp}");

    // Regenerate the version string when the checked-out commit, the active
    // ref, or the staging state changes. Without this the embedded SHA could
    // go stale across incremental rebuilds.
    if let Some(git_dir) = git(&["rev-parse", "--absolute-git-dir"]) {
        println!("cargo:rerun-if-changed={git_dir}/HEAD");
        println!("cargo:rerun-if-changed={git_dir}/index");
        if let Some(reference) = git(&["symbolic-ref", "--quiet", "HEAD"]) {
            println!("cargo:rerun-if-changed={git_dir}/{reference}");
        }
        let packed = format!("{git_dir}/packed-refs");
        if Path::new(&packed).exists() {
            println!("cargo:rerun-if-changed={packed}");
        }
    }
}

/// Run a git command, returning trimmed stdout on success, or `None` when git
/// is absent, this is not a repository, or the output is empty.
fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
