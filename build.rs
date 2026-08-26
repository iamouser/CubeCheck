use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let assets_src = manifest_dir.join("assets");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/cubecheck.ico");
    println!("cargo:rerun-if-changed=assets/tools.json");

    embed_windows_icon(&assets_src);
    encode_window_icon(&assets_src);
    copy_runtime_assets(&assets_src, &profile_dir(&manifest_dir, &profile).join("assets"));
    stage_setup_payload(&profile_dir(&manifest_dir, &profile));
}

fn profile_dir(manifest_dir: &Path, profile: &str) -> PathBuf {
    let root = env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir.join("target"));
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

fn stage_setup_payload(target_dir: &Path) {
    if !target_is_windows() {
        return;
    }
    println!("cargo:rerun-if-env-changed=CUBECHECK_SETUP_PAYLOAD");
    let src = match env::var("CUBECHECK_SETUP_PAYLOAD") {
        Ok(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => target_dir.join("cubecheck.exe"),
    };
    println!("cargo:rerun-if-changed={}", src.display());
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("cubecheck_payload.exe");
    if src.exists() {
        if let Err(e) = fs::copy(&src, &out) {
            println!("cargo:warning=failed to stage cubecheck.exe for setup: {e}");
            let _ = fs::write(&out, []);
        }
    } else {
        // Never keep a leftover payload from a previous OUT_DIR reuse.
        let _ = fs::write(&out, []);
        println!("cargo:warning=cubecheck.exe ещё нет — сначала cargo build --release --bin cubecheck");
    }
}
