//! Host helper: run `zig cc`/`zig ar` while dropping Clang-style Rust triples
//! (`--target=x86_64-unknown-linux-gnu`) that Zig 0.16 cannot parse.
//! Built by scripts/build.ps1 into `.zig-wrappers/zig-cc-filter.exe`.

use std::env;
use std::process::{Command, exit};

fn is_foreign_clang_target(s: &str) -> bool {
    let t = s.trim().trim_matches('"');
    t.contains("unknown-linux")
        || t.contains("apple-darwin")
        || t.contains("apple-macosx")
        || t.contains("apple-ios")
        || t.contains("pc-windows")
        || t.contains("unknown-none")
        || (t.contains("-linux-") && t.contains("unknown"))
}

fn main() {
    let mut argv = env::args().skip(1);
    let zig = argv.next().unwrap_or_else(|| usage());
    let mode = argv.next().unwrap_or_else(|| usage());
    let mut cmd = Command::new(&zig);
    cmd.arg(&mode);
    if mode == "cc" || mode == "c++" || mode == "cxx" {
        let zt = argv.next().unwrap_or_else(|| usage());
        cmd.args(["-target", &zt]);
    }

    let rest: Vec<String> = argv.collect();
    let mut i = 0;
    while i < rest.len() {
        let a = &rest[i];
        if a == "-fno-sanitize=all" {
            i += 1;
            continue;
        }
        if let Some(t) = a.strip_prefix("--target=") {
            if is_foreign_clang_target(t) {
                i += 1;
                continue;
            }
        }
        if a == "--target" || a == "-target" {
            if i + 1 < rest.len() && is_foreign_clang_target(&rest[i + 1]) {
                i += 2;
                continue;
            }
        }
        cmd.arg(a);
        i += 1;
    }

    let status = cmd.status().unwrap_or_else(|e| {
        eprintln!("zig-cc-filter: failed to spawn {zig}: {e}");
        exit(127);
    });
    exit(status.code().unwrap_or(1));
}

fn usage() -> ! {
    eprintln!("usage: zig-cc-filter <zig.exe> cc <zig-target> [args...]");
    eprintln!("       zig-cc-filter <zig.exe> ar [args...]");
    exit(2);
}
