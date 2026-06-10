//! Alias binary: `uvx facto.run mcp` invokes this thin forwarder.
//!
//! PyPI normalizes `facto.run` to `facto-run` per PEP 503. uvx looks
//! for a script named `facto-run` inside the installed wheel. The
//! main binary is named `facto`, so this wrapper provides the missing
//! `facto-run` entry point: it locates the `facto` binary on disk and
//! delegates, forwarding all arguments.
//!
//! A tiny Rust wrapper (~100 KB stripped) is preferred over either a
//! duplicate ~10 MB binary or a Python shim; maturin rejects mixing
//! `bindings = "bin"` with `[project.scripts]`, which rules out the
//! Python path.
use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

fn main() -> ExitCode {
    let Some(target) = locate_facto() else {
        eprintln!("facto-run: could not locate the `facto` binary next to this wrapper");
        return ExitCode::from(127);
    };

    let args: Vec<_> = env::args_os().skip(1).collect();

    #[cfg(unix)]
    {
        let err = Command::new(&target).args(&args).exec();
        // `exec` only returns on error.
        eprintln!("facto-run: failed to launch {}: {}", target.display(), err);
        ExitCode::from(126)
    }

    #[cfg(not(unix))]
    {
        match Command::new(&target).args(&args).status() {
            Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
            Err(err) => {
                eprintln!("facto-run: failed to spawn {}: {}", target.display(), err);
                ExitCode::from(126)
            }
        }
    }
}

fn locate_facto() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let dir = exe.parent()?;
    let mut facto = dir.join(if cfg!(windows) { "facto.exe" } else { "facto" });
    if facto.exists() {
        return Some(facto);
    }
    facto = dir
        .parent()?
        .join(if cfg!(windows) { "facto.exe" } else { "facto" });
    if facto.exists() {
        return Some(facto);
    }
    None
}
