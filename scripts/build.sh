#!/usr/bin/env bash
# CubeCheck multi-target build (Linux/macOS). Same artifact names as build.bat.
set -euo pipefail

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$ROOT"
DIST="$ROOT/dist"
BUILD="$ROOT/build"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)"
TARGET="${1:-windows-x64}"
HOST="$(rustc -vV 2>/dev/null | sed -n 's/^host: //p' || true)"

mkdir -p "$DIST" "$BUILD"

info() { printf '%s\n' "$*"; }
die() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

relname() { printf 'CubeCheck-%s-%s\n' "$VERSION" "$1"; }

is_elf() {
  [ -f "$1" ] || return 1
  local mag
  mag=$(dd if="$1" bs=4 count=1 2>/dev/null || true)
  [ "$mag" = $'\x7fELF' ]
}

sh_has_elf() {
  # Self-extracting .sh: shebang + gzip payload (ELF is inside the gzip, not raw).
  [ -f "$1" ] || return 1
  [ "$(wc -c < "$1")" -gt 50000 ] || return 1
  local sig
  sig=$(dd if="$1" bs=2 count=1 2>/dev/null || true)
  [ "$sig" = '#!' ] || return 1
  grep -a -q $'\x1f\x8b' "$1"
}

is_deb() {
  [ -f "$1" ] || return 1
  local sig
  sig=$(dd if="$1" bs=7 count=1 2>/dev/null || true)
  [ "$sig" = '!<arch>' ]
}

is_macho() {
  [ -f "$1" ] || return 1
  local hex
  hex=$(dd if="$1" bs=4 count=1 2>/dev/null | od -An -tx1 | tr -d ' \n')
  case "$hex" in
    feedface|cefaedfe|feedfacf|cffaedfe|cafebabe|bebafeca) return 0 ;;
    *) return 1 ;;
  esac
}

clear_build() {
  mkdir -p "$BUILD"
  find "$BUILD" -mindepth 1 -maxdepth 1 -exec rm -rf {} +
}

publish_file() {
  local src=$1 dest=$2
  [ -f "$src" ] || return 1
  cp -f "$src" "$BUILD/$dest"
  info "release: $dest"
}

make_zip() {
  local parent=$1 name=$2 out=$3
  if command -v zip >/dev/null 2>&1; then
    ( cd "$parent" && zip -qr "$out" "$name" )
  elif command -v python3 >/dev/null 2>&1; then
    python3 - "$parent" "$name" "$out" <<'PY'
import os, shutil, sys
parent, name, out = sys.argv[1:4]
base, ext = os.path.splitext(out)
if ext.lower() != ".zip":
    base = out
# shutil.make_archive appends .zip
made = shutil.make_archive(base, "zip", parent, name)
if os.path.abspath(made) != os.path.abspath(out) and os.path.isfile(made):
    os.replace(made, out)
PY
  else
    ( cd "$parent" && tar -a -cf "$out" "$name" )
  fi
}

pack_windows_portable() {
  local distname=$1
  local exe="$DIST/$distname/cubecheck-$distname.exe"
  [ -f "$exe" ] || return 1
  local stage="$DIST/$distname/portable"
  local bundle="$stage/CubeCheck"
  rm -rf "$stage"
  mkdir -p "$bundle/assets"
  cp -f "$exe" "$bundle/cubecheck.exe"
  copy_core_assets "$bundle/assets"
  [ -f "$ROOT/LICENSE.md" ] && cp -f "$ROOT/LICENSE.md" "$bundle/"
  local zip="$DIST/$distname/$(relname "$distname.zip")"
  make_zip "$stage" CubeCheck "$zip"
  printf '%s\n' "$zip"
}

pack_macos_zip() {
  local bin="$DIST/macos-universal/cubecheck-macos-universal"
  is_macho "$bin" || return 1
  local stage="$DIST/macos-universal/portable"
  local bundle="$stage/CubeCheck"
  rm -rf "$stage"
  mkdir -p "$bundle/assets"
  cp -f "$bin" "$bundle/cubecheck"
  chmod +x "$bundle/cubecheck" || true
  if is_macho "$DIST/macos-arm64/cubecheck-macos-arm64"; then
    cp -f "$DIST/macos-arm64/cubecheck-macos-arm64" "$bundle/cubecheck-arm64"
    chmod +x "$bundle/cubecheck-arm64" || true
  fi
  if is_macho "$DIST/macos-x64/cubecheck-macos-x64"; then
    cp -f "$DIST/macos-x64/cubecheck-macos-x64" "$bundle/cubecheck-x64"
    chmod +x "$bundle/cubecheck-x64" || true
  fi
  copy_core_assets "$bundle/assets"
  [ -f "$ROOT/LICENSE.md" ] && cp -f "$ROOT/LICENSE.md" "$bundle/"
  local zip="$DIST/macos-universal/$(relname macos-universal.zip)"
  make_zip "$stage" CubeCheck "$zip"
  printf '%s\n' "$zip"
}

publish_release_assets() {
  clear_build
  local win zip deb tar src
  for win in windows-x64 windows-x86; do
    if [ -f "$DIST/$win/CubeCheck-Setup-$win.exe" ]; then
      publish_file "$DIST/$win/CubeCheck-Setup-$win.exe" "$(relname "$win-setup.exe")" || true
      [ "$win" = windows-x64 ] && publish_file "$DIST/$win/CubeCheck-Setup-$win.exe" "CubeCheck-Setup.exe" || true
    elif [ -f "$DIST/$win/CubeCheck-Setup.exe" ]; then
      publish_file "$DIST/$win/CubeCheck-Setup.exe" "$(relname "$win-setup.exe")" || true
      [ "$win" = windows-x64 ] && publish_file "$DIST/$win/CubeCheck-Setup.exe" "CubeCheck-Setup.exe" || true
    fi
    zip="$(pack_windows_portable "$win" || true)"
    [ -n "${zip:-}" ] && [ -f "$zip" ] && publish_file "$zip" "$(relname "$win.zip")" || true
  done

  if is_deb "$DIST/linux-deb-x64/cubecheck_${VERSION}_amd64.deb" \
     && is_elf "$DIST/linux-deb-x64/pkg/usr/bin/cubecheck"; then
    publish_file "$DIST/linux-deb-x64/cubecheck_${VERSION}_amd64.deb" "$(relname linux-deb-x64.deb)" || true
  fi
  if is_deb "$DIST/linux-deb-x86/cubecheck_${VERSION}_i386.deb" \
     && is_elf "$DIST/linux-deb-x86/pkg/usr/bin/cubecheck"; then
    publish_file "$DIST/linux-deb-x86/cubecheck_${VERSION}_i386.deb" "$(relname linux-deb-x86.deb)" || true
  fi

  pack_linux_sh_if_needed() {
    local elf=$1 sh=$2 kind=$3
    if [ -f "$sh" ] && sh_has_elf "$sh"; then
      return 0
    fi
    if is_elf "$elf"; then
      local stage="$DIST/.sh-stage-$kind"
      rm -rf "$stage"
      mkdir -p "$stage/assets"
      cp -f "$elf" "$stage/cubecheck"
      copy_core_assets "$stage/assets"
      [ -f "$ROOT/LICENSE.md" ] && cp -f "$ROOT/LICENSE.md" "$stage/"
      bash "$ROOT/scripts/pack-linux-sh.sh" "$stage" "$sh" "$kind" "$VERSION" || true
    fi
  }

  pack_linux_sh_if_needed \
    "$DIST/linux-deb-x64/cubecheck-linux-deb-x64" \
    "$DIST/linux-deb-x64/$(relname linux-x64.sh)" linux-x64
  pack_linux_sh_if_needed \
    "$DIST/linux-deb-x86/cubecheck-linux-deb-x86" \
    "$DIST/linux-deb-x86/$(relname linux-x86.sh)" linux-x86

  local sh
  for sh in \
    "$DIST/linux-deb-x64/$(relname linux-x64.sh)" \
    "$DIST/linux-deb-x86/$(relname linux-x86.sh)" \
    "$DIST/linux-universal/$(relname linux-universal.sh)"
  do
    if [ -f "$sh" ] && sh_has_elf "$sh"; then
      publish_file "$sh" "$(basename "$sh")" || true
    fi
  done

  zip="$(pack_macos_zip || true)"
  [ -n "${zip:-}" ] && [ -f "$zip" ] && publish_file "$zip" "$(relname macos-universal.zip)" || true

  local uni
  for uni in universal universal-local; do
    if [ -f "$DIST/$uni/$(relname "$uni.zip")" ]; then
      src="$DIST/$uni/$(relname "$uni.zip")"
    elif [ -f "$DIST/$uni/CubeCheck-$uni.zip" ]; then
      src="$DIST/$uni/CubeCheck-$uni.zip"
    else
      src=""
    fi
    [ -n "$src" ] && publish_file "$src" "$(relname "$uni.zip")" || true
  done
}

need_cargo() {
  command -v cargo >/dev/null || die "Rust/cargo not found. Install from https://rustup.rs"
}

ensure_target() {
  rustup target add "$1" >/dev/null
}

cargo_bin() {
  local triple=$1 name=$2 tdir=${3:-target}
  local exe="$name"
  case "$triple" in *windows*) exe="$name.exe" ;; esac
  if [ "$triple" = "$HOST" ]; then
    printf '%s\n' "$ROOT/$tdir/release/$exe"
  else
    printf '%s\n' "$ROOT/$tdir/$triple/release/$exe"
  fi
}

copy_core_assets() {
  mkdir -p "$1"
  cp -f "$ROOT/assets/tools.json" "$1/"
  [ -f "$ROOT/assets/cubecheck.ico" ] && cp -f "$ROOT/assets/cubecheck.ico" "$1/"
}

vendor_missing() {
  local miss=0
  while IFS= read -r rel; do
    [ -z "$rel" ] && continue
    case "$rel" in \#*) continue ;; esac
    if [ ! -f "$ROOT/assets/$rel" ]; then
      printf '  %s\n' "$rel"
      miss=1
    fi
  done < "$ROOT/scripts/vendor-files.txt"
  return $miss
}

copy_vendor() {
  local dest=$1
  while IFS= read -r rel; do
    [ -z "$rel" ] && continue
    case "$rel" in \#*) continue ;; esac
    mkdir -p "$(dirname "$dest/$rel")"
    cp -f "$ROOT/assets/$rel" "$dest/$rel"
  done < "$ROOT/scripts/vendor-files.txt"
  [ -f "$ROOT/assets/Everything.ini" ] && cp -f "$ROOT/assets/Everything.ini" "$dest/"
}

placeholder() {
  mkdir -p "$1"
  cat > "$1/README.txt" <<EOF
CubeCheck payload '$2' was not built on this machine.
Run: ./build.sh $2
The universal launcher errors clearly if the current OS payload is missing.
EOF
}

build_windows() {
  die "windows-* targets require Windows (build.bat). Host is $HOST"
}

build_linux_gnu() {
  local triple=$1 distname=$2 outname=$3
  ensure_target "$triple"
  mkdir -p "$DIST/$distname"
  local log="$DIST/$distname/compile.log"
  local zig="" zt=""
  if command -v zig >/dev/null 2>&1; then
    zig=$(command -v zig)
    case "$triple" in
      x86_64-unknown-linux-gnu) zt=x86_64-linux-gnu.2.17 ;;
      i686-unknown-linux-gnu) zt=x86-linux-gnu.2.17 ;;
    esac
  fi
  if [ -n "$zig" ] && [ -n "$zt" ]; then
    local lower=${triple//-/_}
    local wrapdir="$ROOT/.zig-wrappers"
    mkdir -p "$wrapdir"
    local cc="$wrapdir/zig-cc-${lower}.sh"
    local ar="$wrapdir/zig-ar-${lower}.sh"
    printf '#!/bin/sh\nexec "%s" cc -target %s "$@"\n' "$zig" "$zt" > "$cc"
    printf '#!/bin/sh\nexec "%s" ar "$@"\n' "$zig" > "$ar"
    chmod +x "$cc" "$ar"
    export "CC_${lower}=$cc" "CXX_${lower}=$cc" "AR_${lower}=$ar"
    export "CARGO_TARGET_$(printf '%s' "$lower" | tr '[:lower:]' '[:upper:]')_LINKER=$cc"
  fi
  export PKG_CONFIG_ALLOW_CROSS=1
  set +e
  if command -v cargo-zigbuild >/dev/null; then
    cargo zigbuild --release -p cubecheck --bin cubecheck --target "$triple" >"$log" 2>&1
  elif command -v cross >/dev/null; then
    cross build --release -p cubecheck --bin cubecheck --target "$triple" >"$log" 2>&1
  else
    cargo build --release -p cubecheck --bin cubecheck --target "$triple" >"$log" 2>&1
  fi
  local st=$?
  set -e
  if [ "$st" -ne 0 ]; then
    tail -n 40 "$log" >&2 || true
    die "Failed to build $triple (exit $st). Install zig / cargo-zigbuild or a Linux GNU toolchain. See $log"
  fi
  local src="$ROOT/target/$triple/release/cubecheck"
  is_elf "$src" || die "missing or not ELF: $src"
  mkdir -p "$DIST/$distname"
  cp -f "$src" "$DIST/$distname/$outname"
  copy_core_assets "$DIST/$distname/assets"
  printf '%s\n' "$DIST/$distname/$outname"
}

write_deb_tree() {
  local arch=$1 distname=$2 linuxbin=${3:-}
  local pkg="$DIST/$distname/pkg"
  rm -rf "$pkg"
  mkdir -p "$pkg/DEBIAN" "$pkg/usr/bin" "$pkg/usr/share/cubecheck/assets" \
    "$pkg/usr/share/doc/cubecheck" "$pkg/usr/share/applications"
  sed -e "s/@VERSION@/$VERSION/g" -e "s/@ARCH@/$arch/g" \
    "$ROOT/scripts/debian/control.in" > "$pkg/DEBIAN/control"
  cp -f "$ROOT/scripts/debian/copyright" "$pkg/usr/share/doc/cubecheck/copyright"
  cp -f "$ROOT/LICENSE.md" "$pkg/usr/share/doc/cubecheck/LICENSE.md"
  copy_core_assets "$pkg/usr/share/cubecheck/assets"
  cat > "$pkg/usr/share/applications/cubecheck.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=CubeCheck
Exec=cubecheck
Terminal=false
Categories=Utility;
EOF
  if [ -n "$linuxbin" ] && is_elf "$linuxbin"; then
    cp -f "$linuxbin" "$pkg/usr/bin/cubecheck"
    chmod 755 "$pkg/usr/bin/cubecheck"
  fi
  cp -f "$ROOT/scripts/pack-deb.sh" "$DIST/$distname/pack-deb.sh"
  printf '%s\n' "$pkg"
}

pack_deb() {
  local pkg=$1 deb=$2
  is_elf "$pkg/usr/bin/cubecheck" || return 1
  bash "$ROOT/scripts/pack-deb.sh" "$pkg" "$deb"
}

build_launcher() {
  cargo build --release -p cubecheck-launcher
  local src="$ROOT/target/release/cubecheck-launcher"
  [ -f "$src.exe" ] && src="$src.exe"
  [ -f "$src" ] || die "launcher not found"
  printf '%s\n' "$src"
}

pack_linux_universal() {
  local bundle="$DIST/linux-universal/CubeCheck-linux-universal"
  rm -rf "$bundle"
  mkdir -p "$bundle/payload"
  cp -f "$ROOT/scripts/posix-launcher.sh" "$bundle/cubecheck"
  cp -f "$ROOT/scripts/posix-launcher.sh" "$bundle/cubecheck.sh"
  chmod +x "$bundle/cubecheck" "$bundle/cubecheck.sh"
  cp -f "$ROOT/LICENSE.md" "$bundle/"
  local have=0
  for pair in "linux-x64 linux-deb-x64 cubecheck-linux-deb-x64" "linux-x86 linux-deb-x86 cubecheck-linux-deb-x86"; do
    set -- $pair
    local id=$1 distname=$2 bin=$3
    if is_elf "$DIST/$distname/$bin"; then
      mkdir -p "$bundle/payload/$id/assets"
      cp -f "$DIST/$distname/$bin" "$bundle/payload/$id/cubecheck"
      chmod +x "$bundle/payload/$id/cubecheck"
      copy_core_assets "$bundle/payload/$id/assets"
      : > "$bundle/payload/$id/.portable"
      have=$((have + 1))
    else
      placeholder "$bundle/payload/$id" "$id"
    fi
  done
  local sh="$DIST/linux-universal/$(relname linux-universal.sh)"
  rm -f "$sh"
  [ "$have" -gt 0 ] || {
    cat > "$DIST/linux-universal/README.txt" <<EOF
No Linux cubecheck ELF was produced on this host.
Do not ship a dummy .sh. Build linux-deb-x64 / linux-deb-x86 first.
This layout stays in dist/; it is not copied to build/ (GitHub Releases).
EOF
    return 1
  }
  local stage="$DIST/linux-universal/sh-stage"
  rm -rf "$stage"
  mkdir -p "$stage"
  cp -a "$bundle/payload" "$stage/payload"
  copy_core_assets "$stage/assets"
  cp -f "$ROOT/LICENSE.md" "$stage/"
  bash "$ROOT/scripts/pack-linux-sh.sh" "$stage" "$sh" linux-universal "$VERSION"
  info "$bundle"
}

pack_universal() {
  local local_sku=$1
  local name folder
  if [ "$local_sku" = 1 ]; then
    name=universal-local
    folder=CubeCheck-universal-local
  else
    name=universal
    folder=CubeCheck-universal
  fi
  local bundle="$DIST/$name/$folder"
  rm -rf "$bundle"
  mkdir -p "$bundle/payload"
  local launcher
  launcher="$(build_launcher)"
  case "$launcher" in
    *.exe) cp -f "$launcher" "$bundle/cubecheck.exe" ;;
    *) cp -f "$launcher" "$bundle/cubecheck" ; chmod +x "$bundle/cubecheck" ;;
  esac
  cp -f "$ROOT/scripts/posix-launcher.sh" "$bundle/cubecheck.sh"
  cp -f "$ROOT/scripts/cubecheck.command" "$bundle/cubecheck.command"
  chmod +x "$bundle/cubecheck.sh" "$bundle/cubecheck.command" || true
  cp -f "$ROOT/LICENSE.md" "$bundle/"
  [ "$local_sku" = 1 ] && : > "$bundle/.offline"

  put_win() {
    local id=$1 src=$2
    if [ -f "$src" ]; then
      mkdir -p "$bundle/payload/$id/assets"
      cp -f "$src" "$bundle/payload/$id/cubecheck.exe"
      copy_core_assets "$bundle/payload/$id/assets"
      : > "$bundle/payload/$id/.portable"
      if [ "$local_sku" = 1 ]; then
        : > "$bundle/payload/$id/.offline"
        copy_vendor "$bundle/payload/$id/assets"
      fi
      return 0
    fi
    placeholder "$bundle/payload/$id" "$id"
    return 1
  }

  local have=0
  put_win windows-x64 "$DIST/windows-x64/cubecheck-windows-x64.exe" && have=$((have+1)) || true
  put_win windows-x86 "$DIST/windows-x86/cubecheck-windows-x86.exe" && have=$((have+1)) || true

  for pair in "linux-x64 linux-deb-x64 cubecheck-linux-deb-x64" "linux-x86 linux-deb-x86 cubecheck-linux-deb-x86"; do
    set -- $pair
    local id=$1 distname=$2 bin=$3
    if is_elf "$DIST/$distname/$bin"; then
      mkdir -p "$bundle/payload/$id/assets"
      cp -f "$DIST/$distname/$bin" "$bundle/payload/$id/cubecheck"
      chmod +x "$bundle/payload/$id/cubecheck"
      copy_core_assets "$bundle/payload/$id/assets"
      : > "$bundle/payload/$id/.portable"
      [ "$local_sku" = 1 ] && : > "$bundle/payload/$id/.offline"
      have=$((have+1))
    else
      placeholder "$bundle/payload/$id" "$id"
    fi
  done
  if is_macho "$DIST/macos-universal/cubecheck-macos-universal"; then
    mkdir -p "$bundle/payload/macos-universal/assets"
    cp -f "$DIST/macos-universal/cubecheck-macos-universal" "$bundle/payload/macos-universal/cubecheck"
    copy_core_assets "$bundle/payload/macos-universal/assets"
    : > "$bundle/payload/macos-universal/.portable"
    have=$((have+1))
  else
    placeholder "$bundle/payload/macos-universal" macos-universal
  fi
  [ "$have" -gt 0 ] || die "$name: no payload binaries"
  make_zip "$DIST/$name" "$folder" "$DIST/$name/$folder.zip"
  info "$bundle"
}

build_macos() {
  mkdir -p "$DIST/macos-universal"
  local x64="" arm=""
  build_darwin_slice() {
    local triple=$1 distname=$2
    ensure_target "$triple" || return 1
    mkdir -p "$DIST/$distname"
    local log="$DIST/$distname/compile.log"
    local zig zt=""
    if command -v zig >/dev/null 2>&1; then
      zig=$(command -v zig)
      case "$triple" in
        x86_64-apple-darwin) zt=x86_64-macos ;;
        aarch64-apple-darwin) zt=aarch64-macos ;;
      esac
    fi
    if [ -n "${zig:-}" ] && [ -n "$zt" ]; then
      local lower=${triple//-/_}
      local wrapdir="$ROOT/.zig-wrappers"
      mkdir -p "$wrapdir"
      local cc="$wrapdir/zig-cc-${lower}.sh"
      local extra=""
      if [ -n "${SDKROOT:-}" ]; then extra="-isysroot $SDKROOT"; fi
      printf '#!/bin/sh\nexec "%s" cc -target %s %s "$@"\n' "$zig" "$zt" "$extra" > "$cc"
      chmod +x "$cc"
      export "CC_${lower}=$cc" "CXX_${lower}=$cc"
      export "CARGO_TARGET_$(printf '%s' "$lower" | tr '[:lower:]' '[:upper:]')_LINKER=$cc"
    fi
    set +e
    cargo build --release -p cubecheck --bin cubecheck --target "$triple" >"$log" 2>&1
    local st=$?
    set -e
    if [ "$st" -eq 0 ]; then
      local src="$ROOT/target/$triple/release/cubecheck"
      if is_macho "$src"; then
        mkdir -p "$DIST/$distname"
        cp -f "$src" "$DIST/$distname/cubecheck-$distname"
        printf '%s\n' "$DIST/$distname/cubecheck-$distname"
        return 0
      fi
    fi
    return 1
  }
  x64=$(build_darwin_slice x86_64-apple-darwin macos-x64 || true)
  arm=$(build_darwin_slice aarch64-apple-darwin macos-arm64 || true)
  if [ -z "$x64" ] && [ -z "$arm" ]; then
    placeholder "$DIST/macos-universal/payload" macos-universal
    cat > "$DIST/macos-universal/README.txt" <<'EOF'
Need a macOS SDK / Apple linker. Do not download Xcode SDKs from random mirrors.

On a Mac:
  rustup target add x86_64-apple-darwin aarch64-apple-darwin
  cargo build --release --bin cubecheck --target x86_64-apple-darwin
  cargo build --release --bin cubecheck --target aarch64-apple-darwin
  lipo -create -output dist/macos-universal/cubecheck-macos-universal \
    target/x86_64-apple-darwin/release/cubecheck \
    target/aarch64-apple-darwin/release/cubecheck
  ./build.sh macos-universal

GitHub asset: build/CubeCheck-*-macos-universal.zip with inner Mach-O named cubecheck (no extension).
chmod +x cubecheck
EOF
    die "macos-universal: cannot produce Mach-O on this host (see dist/macos-*/compile.log)"
  fi
  local dest="$DIST/macos-universal/cubecheck-macos-universal"
  if [ -n "$x64" ] && [ -n "$arm" ] && command -v lipo >/dev/null; then
    lipo -create -output "$dest" "$x64" "$arm"
  elif [ -n "$arm" ]; then
    cp -f "$arm" "$dest"
  else
    cp -f "$x64" "$dest"
  fi
  is_macho "$dest" || die "macos-universal output is not Mach-O"
  chmod +x "$dest" || true
  copy_core_assets "$DIST/macos-universal/assets"
  info "$dest"
}

usage() {
  cat <<'EOF'
CubeCheck build

  ./build.sh                 host GUI if this is Windows (else linux-deb-x64)
  ./build.sh all
  ./build.sh publish         rebuild build/ from dist/ (no compile)
  ./build.sh windows-x64 | windows-x86 | linux-deb-x64 | linux-deb-x86
  ./build.sh linux-universal | macos-universal | universal | universal-local

GitHub Release assets (build/): versioned .exe / .zip / .sh / .deb only.
Linux asset is CubeCheck-*-linux-x64.sh (chmod +x; run it). macOS is a zip
with inner Mach-O named cubecheck (chmod +x). Published only when a real
ELF/Mach-O exists. Staging stays in dist/.
EOF
}

case "$TARGET" in
  help|-h|--help) usage; exit 0 ;;
  publish)
    info "========================================"
    info " CubeCheck  ($VERSION) — publish only"
    info "========================================"
    publish_release_assets
    info ""
    info "build/ (GitHub Release assets):"
    if [ -n "$(ls -A "$BUILD" 2>/dev/null || true)" ]; then
      ls -lh "$BUILD"
    else
      info "  (empty)"
    fi
    info "Done."
    exit 0
    ;;
esac

need_cargo

info "========================================"
info " CubeCheck  ($VERSION)"
info "========================================"

run_one() {
  local t=$1
  set +e
  run_one_inner "$t"
  local st=$?
  set -e
  publish_release_assets || true
  return $st
}

run_one_inner() {
  case "$1" in
    windows-x64|windows-x86) build_windows ;;
    linux-deb-x64)
      bin="$(build_linux_gnu x86_64-unknown-linux-gnu linux-deb-x64 cubecheck-linux-deb-x64)"
      pkg="$(write_deb_tree amd64 linux-deb-x64 "$bin")"
      pack_deb "$pkg" "$DIST/linux-deb-x64/cubecheck_${VERSION}_amd64.deb" || true
      stage="$DIST/linux-deb-x64/sh-stage"
      rm -rf "$stage"; mkdir -p "$stage/assets"
      cp -f "$bin" "$stage/cubecheck"
      copy_core_assets "$stage/assets"
      [ -f "$ROOT/LICENSE.md" ] && cp -f "$ROOT/LICENSE.md" "$stage/"
      bash "$ROOT/scripts/pack-linux-sh.sh" "$stage" "$DIST/linux-deb-x64/$(relname linux-x64.sh)" linux-x64 "$VERSION"
      ;;
    linux-deb-x86)
      bin="$(build_linux_gnu i686-unknown-linux-gnu linux-deb-x86 cubecheck-linux-deb-x86)"
      pkg="$(write_deb_tree i386 linux-deb-x86 "$bin")"
      pack_deb "$pkg" "$DIST/linux-deb-x86/cubecheck_${VERSION}_i386.deb" || true
      stage="$DIST/linux-deb-x86/sh-stage"
      rm -rf "$stage"; mkdir -p "$stage/assets"
      cp -f "$bin" "$stage/cubecheck"
      copy_core_assets "$stage/assets"
      [ -f "$ROOT/LICENSE.md" ] && cp -f "$ROOT/LICENSE.md" "$stage/"
      bash "$ROOT/scripts/pack-linux-sh.sh" "$stage" "$DIST/linux-deb-x86/$(relname linux-x86.sh)" linux-x86 "$VERSION"
      ;;
    linux-universal) pack_linux_universal ;;
    macos-universal) build_macos ;;
    universal) pack_universal 0 ;;
    universal-local)
      check_vendor_or_die
      pack_universal 1
      ;;
    *) die "Unknown target $1" ;;
  esac
}

check_vendor_or_die() {
  local list=""
  while IFS= read -r rel; do
    [ -z "$rel" ] && continue
    case "$rel" in \#*) continue ;; esac
    [ -f "$ROOT/assets/$rel" ] || list="$list  $rel"$'\n'
  done < "$ROOT/scripts/vendor-files.txt"
  if [ -n "$list" ]; then
    printf 'universal-local: missing vendor files in assets/:\n%s' "$list"
    die "Download tools once with a normal Windows build, then retry."
  fi
}

if [ "$TARGET" = "all" ]; then
  failed=0
  for t in linux-deb-x64 linux-deb-x86 linux-universal macos-universal universal universal-local; do
    info ""
    info "---- $t ----"
    if ! run_one "$t"; then failed=1; fi
  done
  [ "$failed" -eq 0 ] || exit 1
  exit 0
fi

if ! run_one "$TARGET"; then
  exit 1
fi
info "Done."
