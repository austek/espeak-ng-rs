//! Shared helpers: package the library as `libespeak-ng.so`, build the C
//! probe against it, run scenarios and read the fixtures recorded from the
//! reference C library.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn data_root() -> PathBuf {
    std::env::var_os("ESPEAK_DATA_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("..").join("espeak-ng-data"))
}

pub fn fixture_path(name: &str) -> PathBuf {
    manifest_dir().join("tests").join("fixtures").join(name)
}

pub fn fixture(name: &str) -> String {
    std::fs::read_to_string(fixture_path(name))
        .unwrap_or_else(|e| panic!("fixture {name}: {e}"))
}

fn tmp() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
}

/// The profile directory holding the library, after building it there.
///
/// `cargo test` does not build a package's cdylib in every profile, so the
/// helper runs `cargo build` for the profile the tests were compiled in
/// (`debug_assertions` on means debug), once.
pub fn profile_dir() -> &'static PathBuf {
    static PROFILE: OnceLock<PathBuf> = OnceLock::new();
    PROFILE.get_or_init(|| {
        // The test binary's own location is not reliable (newer cargo keeps
        // it under `target/<profile>/build/`), so derive both from stable facts.
        let target = tmp().parent().expect("target dir").to_path_buf();
        let release = !cfg!(debug_assertions);
        let dir = target.join(if release { "release" } else { "debug" });
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let mut build = Command::new(cargo);
        build
            .args(["build", "-p", "espeak-ng-capi", "--target-dir"])
            .arg(target)
            .arg("--manifest-path")
            .arg(manifest_dir().join("Cargo.toml"));
        if dir.file_name().is_some_and(|n| n == "release") {
            build.arg("--release");
        }
        assert!(build.status().expect("run cargo build").success(), "cargo build failed");
        dir
    })
}

fn built_library() -> PathBuf {
    let lib = profile_dir().join("libespeak_ng_capi.so");
    assert!(lib.exists(), "library not built: {}", lib.display());
    lib
}

/// Install the cdylib as `libespeak-ng.so.1` plus the `.so` link, as
/// `scripts/package.sh` does.
pub fn dist_dir() -> &'static PathBuf {
    static DIST: OnceLock<PathBuf> = OnceLock::new();
    DIST.get_or_init(|| {
        let dist = tmp().join("dist");
        std::fs::create_dir_all(&dist).expect("dist dir");
        let versioned = dist.join("libespeak-ng.so.1");
        std::fs::copy(built_library(), &versioned).expect("copy library");
        let link = dist.join("libespeak-ng.so");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink("libespeak-ng.so.1", &link).expect("symlink");
        dist
    })
}

pub fn compile_c(source: &Path, out: &Path, extra: &[&str]) {
    let status = Command::new("cc")
        .arg("-O1")
        .arg("-o")
        .arg(out)
        .arg(source)
        .arg("-I")
        .arg(manifest_dir().join("include"))
        .args(extra)
        .status()
        .expect("run cc");
    assert!(status.success(), "cc failed for {}", source.display());
}

pub fn probe_binary() -> &'static PathBuf {
    static PROBE: OnceLock<PathBuf> = OnceLock::new();
    PROBE.get_or_init(|| {
        let dist = dist_dir();
        let out = tmp().join("probe_rs");
        let rpath = format!("-Wl,-rpath,{}", dist.display());
        let libdir = format!("-L{}", dist.display());
        compile_c(
            &manifest_dir().join("tests/c/probe.c"),
            &out,
            &[&libdir, "-lespeak-ng", &rpath],
        );
        out
    })
}

pub fn run_probe_with(binary: &Path, args: &[&str]) -> String {
    let out = Command::new(binary)
        .args(args)
        .env("ESPEAK_DATA_PATH", data_root())
        .output()
        .expect("run probe");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

pub fn run_probe(args: &[&str]) -> String {
    run_probe_with(probe_binary(), args)
}

pub fn have_data() -> bool {
    data_root().join("espeak-ng-data").join("phontab").exists()
        || data_root().join("phontab").exists()
}

/// FNV-1a over the little-endian bytes of the samples, as the probe computes.
pub fn fnv_samples(samples: &[i16]) -> u64 {
    let mut h: u64 = 1469598103934665603;
    for b in samples.iter().flat_map(|s| s.to_le_bytes()) {
        h ^= u64::from(b);
        h = h.wrapping_mul(1099511628211);
    }
    h
}
