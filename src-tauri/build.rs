fn main() {
    let attrs = tauri_build::Attributes::new();
    let profile = std::env::var("PROFILE").unwrap_or_default();

    if profile == "release" {
        tauri_build::try_build(attrs).expect("failed to run tauri-build");
    } else {
        if let Err(e) = tauri_build::try_build(attrs) {
            println!("cargo:warning=tauri-build partial (dev profile): {e}");
            // Embed Windows manifest for comctl32 v6 activation and DPI awareness
            embed_windows_manifest();
        }
    }
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
