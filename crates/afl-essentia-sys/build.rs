use std::env;
use std::path::{Path, PathBuf};

fn main() {
    if env::var_os("CARGO_FEATURE_NATIVE_BACKEND").is_none() {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("workspace root");
    let native_wrapper_dir = workspace_root.join("native/essentia-wrapper");

    println!("cargo:rerun-if-changed={}", native_wrapper_dir.display());
    println!("cargo:rerun-if-env-changed=ESSENTIA_PREFIX");
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");

    let essentia_prefix = resolve_essentia_prefix();
    let pkg_config_path = join_pkg_config_path(&essentia_prefix);
    unsafe {
        env::set_var("ESSENTIA_PREFIX", &essentia_prefix);
        env::set_var("PKG_CONFIG_PATH", &pkg_config_path);
    }

    let library = pkg_config::Config::new()
        .statik(false)
        .env_metadata(true)
        .cargo_metadata(true)
        .probe("essentia")
        .unwrap_or_else(|error| {
            panic!(
                "failed to locate Essentia via pkg-config using ESSENTIA_PREFIX={}: {error}",
                essentia_prefix.display()
            )
        });

    let mut build = cmake::Config::new(&native_wrapper_dir);
    build.define("ESSENTIA_PREFIX", &essentia_prefix);
    build.define(
        "ESSENTIA_EXTRA_INCLUDE_DIRS",
        library
            .include_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(";"),
    );
    build.profile("Release");
    let dst = build.build();

    println!(
        "cargo:rustc-link-search=native={}",
        dst.join("lib").display()
    );
    println!("cargo:rustc-link-lib=static=afl_essentia_wrapper");

    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=framework=Accelerate");
        println!("cargo:rustc-link-lib=c++");
    } else if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=stdc++");
    }

    for include_path in &library.include_paths {
        println!("cargo:include={}", include_path.display());
    }
}

fn resolve_essentia_prefix() -> PathBuf {
    if let Some(value) = env::var_os("ESSENTIA_PREFIX") {
        return PathBuf::from(value);
    }

    let fallback = PathBuf::from("/tmp/essentia-install");
    if fallback.join("lib/pkgconfig/essentia.pc").is_file() {
        println!(
            "cargo:warning=ESSENTIA_PREFIX not set, using local fallback {}",
            fallback.display()
        );
        return fallback;
    }

    panic!(
        "native-backend requires Essentia. Set ESSENTIA_PREFIX to the Essentia installation prefix"
    );
}

fn join_pkg_config_path(prefix: &Path) -> String {
    let mut paths = vec![prefix.join("lib/pkgconfig")];
    if let Some(existing) = env::var_os("PKG_CONFIG_PATH") {
        paths.extend(env::split_paths(&existing));
    }

    env::join_paths(paths)
        .expect("valid PKG_CONFIG_PATH")
        .to_string_lossy()
        .into_owned()
}
