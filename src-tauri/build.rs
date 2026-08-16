fn main() {
    let attrs = tauri_build::Attributes::new();
    let profile = std::env::var("PROFILE").unwrap_or_default();

    // ── Inject BUILD_NUMBER for compile-time version stamping ────────
    // In release builds the build number is auto-incremented from a
    // starting value of `BUILD_START` (see below) and persisted to the
    // repo-root `BUILD` file. In dev builds we just read it for display
    // without mutating it.
    let build_number = inject_build_number(&profile);
    println!("cargo:rerun-if-changed=../BUILD");

    if profile == "release" {
        tauri_build::try_build(attrs).expect("failed to run tauri-build");
    } else {
        if let Err(e) = tauri_build::try_build(attrs) {
            println!("cargo:warning=tauri-build partial (dev profile): {e}");
            // Embed Windows manifest for comctl32 v6 activation and DPI awareness
            embed_windows_manifest();
        }
    }

    // Touch the chosen number into the build output so CI logs show it.
    println!("cargo:warning=2-Pyramid build number: {}", build_number);

    // ALWAYS inject BUILD_NUMBER, regardless of profile. The crate
    // reads it via `env!("BUILD_NUMBER")` so it must be set at compile
    // time. Release builds use the auto-bumped value; debug builds use
    // the `dev.N` form (which also surfaces a useful marker in the UI).
    println!("cargo:rustc-env=BUILD_NUMBER={}", build_number);
}

/// First build number to use when the repo-root `BUILD` file is
/// missing or contains a value below this floor. Chosen so it leaves
/// plenty of headroom for hotfix builds while still being clearly
/// distinguishable from any legacy 1.x builds.
const BUILD_START: u64 = 20000;

/// Reads the repo-root `BUILD` file and decides what value to expose
/// to the crate as `BUILD_NUMBER`.
///
/// Release behaviour:
///   * If BUILD is missing or below `BUILD_START`, seed it with
///     `BUILD_START` (no increment).
///   * Otherwise, atomically increment the value in the file and
///     return the new number.
///   * The marker file `.BUILD_LAST_USED` is touched so that if Cargo
///     re-invokes this `build.rs` for the same source tree (which it
///     does during build-script probing), we don't double-bump.
///
/// Dev behaviour:
///   * Read the current value and return it prefixed with `"dev."` so
///     the UI can show a useful marker without mutating the file.
fn inject_build_number(profile: &str) -> String {
    // CARGO_MANIFEST_DIR points at src-tauri/, so the BUILD file is one level up.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let build_file = std::path::Path::new(&manifest_dir).join("..").join("BUILD");
    let marker_file = std::path::Path::new(&manifest_dir).join("..").join(".BUILD_LAST_USED");

    let current = read_build_number(&build_file);

    if profile == "release" {
        // First build (no file or below floor) -> seed at BUILD_START.
        // Otherwise bump and persist.
        let new_value = match current {
            None => BUILD_START,
            Some(v) if v < BUILD_START => BUILD_START,
            Some(v) => v + 1,
        };

        // Atomic-ish write: write to temp then rename, so a crashing
        // build script never leaves a half-written BUILD file.
        if let Err(e) = atomic_write(&build_file, &new_value.to_string()) {
            println!("cargo:warning=failed to update BUILD file: {}", e);
        }

        // Touch the marker so the rerun guard below sees a fresh
        // timestamp and Cargo will rerun build.rs on the next release.
        let _ = std::fs::write(&marker_file, new_value.to_string());

        new_value.to_string()
    } else {
        // Dev: just read, prefix with "dev." for the UI to display.
        match current {
            Some(v) => format!("dev.{}", v),
            None => "dev".to_string(),
        }
    }
}

// (the `value` computed by `inject_build_number` is captured into
//  `BUILD_NUMBER` for the crate via the `cargo:rustc-env` line emitted
//  at the bottom of this file.)

/// Read the BUILD file and return its value if it is a non-empty
/// integer. Returns `None` for missing / empty / malformed files.
fn read_build_number(path: &std::path::Path) -> Option<u64> {
    let raw = std::fs::read_to_string(path).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<u64>().ok()
}

/// Write the build number through a temp file + rename so concurrent
/// cargo invocations don't truncate the file.
fn atomic_write(path: &std::path::Path, value: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("BUILD.tmp");
    std::fs::write(&tmp, format!("{}\n", value))?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn embed_windows_manifest() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let manifest_path = std::path::Path::new(&out_dir).join("app.manifest");

    let manifest = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0"
        processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
    </dependentAssembly>
  </dependency>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </windowsSettings>
  </application>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
      <supportedOS Id="{1f676c76-80e1-4239-95bb-83d0f6d0da78}"/>
    </application>
  </compatibility>
</assembly>"#;

    std::fs::write(&manifest_path, manifest).expect("failed to write manifest");
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg=/MANIFESTINPUT:{}",
        manifest_path.display()
    );
}

#[cfg(not(target_os = "windows"))]
fn embed_windows_manifest() {}
