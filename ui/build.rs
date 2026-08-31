use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo = manifest_dir.parent().unwrap_or(&manifest_dir).to_path_buf();
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let assets_src = repo.join("assets");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../assets/cubecheck.ico");
    println!("cargo:rerun-if-changed=../assets/tools.json");

    embed_windows_icon(&assets_src);
    encode_window_icon(&assets_src);
    let profile_dir = profile_dir(&manifest_dir, &profile);
    copy_runtime_assets(&assets_src, &profile_dir.join("assets"));
    stage_host_binaries(&repo, &profile_dir);
    stage_setup_payload(&profile_dir);
}

fn profile_dir(manifest_dir: &Path, profile: &str) -> PathBuf {
    let root = env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            manifest_dir
                .parent()
                .map(|p| p.join("target"))
                .unwrap_or_else(|| manifest_dir.join("target"))
        });
    let host = env::var("HOST").unwrap_or_default();
    let target = env::var("TARGET").unwrap_or_default();
    if !target.is_empty() && host != target {
        root.join(target).join(profile)
    } else {
        root.join(profile)
    }
}

fn target_is_windows() -> bool {
    env::var("CARGO_CFG_TARGET_OS").ok().as_deref() == Some("windows")
}

fn embed_windows_icon(assets: &Path) {
    if !target_is_windows() {
        return;
    }

    let icon = assets.join("cubecheck.ico");
    if !icon.exists() {
        println!("cargo:warning=assets/cubecheck.ico not found");
        return;
    }

    if let Err(e) = winres::WindowsResource::new()
        .set_icon(icon.to_str().unwrap_or("assets/cubecheck.ico"))
        .set("ProductName", "CubeCheck")
        .set("FileDescription", "CubeCheck")
        .set("FileVersion", "1.1.0")
        .set("ProductVersion", "1.1 beta")
        .set("CompanyName", "AuraStudio, AnProject")
        .set("LegalCopyright", "Copyright (c) 2026 AuraStudio, AnProject")
        .compile()
    {
        println!("cargo:warning=failed to embed icon: {e}");
    }
}

fn encode_window_icon(assets: &Path) {
    let icon = assets.join("cubecheck.ico");
    let img = image::open(&icon)
        .unwrap_or_else(|e| panic!("failed to decode {}: {e}", icon.display()))
        .to_rgba8();
    let (width, height) = img.dimensions();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    fs::write(out.join("icon_rgba.bin"), img.as_raw()).expect("write icon_rgba.bin");
    println!("cargo:rustc-env=ICON_WIDTH={width}");
    println!("cargo:rustc-env=ICON_HEIGHT={height}");
}

fn copy_runtime_assets(src: &Path, dst: &Path) {
    if let Err(e) = fs::create_dir_all(dst) {
        println!("cargo:warning=failed to create {}: {e}", dst.display());
        return;
    }
    for name in ["cubecheck.ico", "tools.json"] {
        let from = src.join(name);
        if !from.exists() {
            continue;
        }
        if let Err(e) = copy_if_changed(&from, &dst.join(name)) {
            println!("cargo:warning=failed to copy {name}: {e}");
        }
    }
}

fn copy_if_changed(src: &Path, dst: &Path) -> std::io::Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    let needs_copy = match fs::metadata(dst) {
        Ok(meta) => meta.len() != fs::metadata(src)?.len(),
        Err(_) => true,
    };
    if needs_copy {
        fs::copy(src, dst)?;
    }
    Ok(())
}

fn stage_file(src: &Path, dest: &Path, missing_note: &str) {
    println!("cargo:rerun-if-changed={}", src.display());
    if src.is_file() {
        if let Err(e) = fs::copy(src, dest) {
            println!("cargo:warning=не удалось скопировать {}: {e}", src.display());
            let _ = fs::write(dest, []);
        }
    } else {
        let _ = fs::write(dest, []);
        println!("cargo:warning={missing_note}");
    }
}

fn first_existing(candidates: &[PathBuf]) -> PathBuf {
    candidates
        .iter()
        .find(|p| p.is_file())
        .cloned()
        .unwrap_or_else(|| candidates[0].clone())
}

fn stage_host_binaries(repo: &Path, target_dir: &Path) {
    println!("cargo:rerun-if-env-changed=CUBECHECK_API_DLL");
    println!("cargo:rerun-if-env-changed=CUBECHECK_NATIVE_DLL");

    let api_src = match env::var("CUBECHECK_API_DLL") {
        Ok(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => first_existing(&[
            repo.join("src/CubeCheck.Api/bin/Release/net8.0/win-x64/publish/cubecheck_api.dll"),
            repo.join("src/CubeCheck.Api/bin/Release/net8.0/win-x64/native/cubecheck_api.dll"),
            target_dir.join("assets").join("cubecheck_api.dll"),
            repo.join("assets").join("cubecheck_api.dll"),
            target_dir.join("cubecheck_api.dll"),
        ]),
    };
    let native_src = match env::var("CUBECHECK_NATIVE_DLL") {
        Ok(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => first_existing(&[
            repo.join("src/native/bin/x64/cubecheck_native.dll"),
            repo.join("src/native/bin/cubecheck_native.dll"),
            target_dir.join("assets").join("cubecheck_native.dll"),
            repo.join("assets").join("cubecheck_native.dll"),
            target_dir.join("cubecheck_native.dll"),
        ]),
    };

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    stage_file(
        &api_src,
        &out.join("cubecheck_api.dll"),
        "cubecheck_api.dll ещё нет — сначала dotnet publish CubeCheck.Api",
    );
    stage_file(
        &native_src,
        &out.join("cubecheck_native.dll"),
        "cubecheck_native.dll ещё нет — сначала соберите C++ native",
    );

    let assets_dir = target_dir.join("assets");
    let _ = fs::create_dir_all(&assets_dir);
    if api_src.is_file() {
        let _ = fs::copy(&api_src, assets_dir.join("cubecheck_api.dll"));
    }
    if native_src.is_file() {
        let _ = fs::copy(&native_src, assets_dir.join("cubecheck_native.dll"));
    }
    for name in [
        "cubecheck_api.dll",
        "cubecheck_native.dll",
        "UnInstall.ico",
        "UnInstall.cmd",
    ] {
        let leftover = target_dir.join(name);
        if leftover.is_file() {
            let _ = fs::remove_file(leftover);
        }
    }
}

fn stage_setup_payload(target_dir: &Path) {
    if !target_is_windows() {
        return;
    }
    println!("cargo:rerun-if-env-changed=CUBECHECK_SETUP_ZIP");
    println!("cargo:rerun-if-env-changed=CUBECHECK_SETUP_PAYLOAD");
    let src = match env::var("CUBECHECK_SETUP_ZIP") {
        Ok(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => match env::var("CUBECHECK_SETUP_PAYLOAD") {
            Ok(p) if !p.trim().is_empty() => PathBuf::from(p),
            _ => first_existing(&[
                target_dir.join("universal-windows-payload.zip"),
                target_dir.join("cubecheck.exe"),
            ]),
        },
    };
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    stage_file(
        &src,
        &out.join("universal_payload.zip"),
        "нет zip универсального payload — сначала scripts/build.ps1 (CUBECHECK_SETUP_ZIP)",
    );
    // leftover name so older include_bytes still compile if a crate is mid-rebuild
    stage_file(
        &src,
        &out.join("cubecheck_payload.exe"),
        "cubecheck.exe ещё нет — сначала cargo build --release --bin cubecheck",
    );
}
