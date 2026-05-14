fn main() {
    println!("cargo:rerun-if-changed=src/app.ico");
    println!("cargo:rerun-if-changed=Cargo.toml");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    configure_gnu_resource_tools();

    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon("src/app.ico")
        .set("FileDescription", "MuteApp")
        .set("ProductName", "MuteApp")
        .set("OriginalFilename", "MuteApp.exe")
        .set("InternalName", "MuteApp")
        .set(
            "Comments",
            "Mute or unmute the foreground application with a global hotkey.",
        );
    resource
        .compile()
        .expect("failed to compile Windows resource");
}

fn configure_gnu_resource_tools() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.ends_with("-gnu") {
        return;
    }

    const WINDRES: &str = "x86_64-w64-mingw32-windres";
    const AR: &str = "x86_64-w64-mingw32-ar";
    if !command_exists(WINDRES) || !command_exists(AR) {
        println!("cargo:warning=skipping Windows resource metadata; MinGW windres/ar not found");
        std::process::exit(0);
    }

    // The resource compiler crate reads CARGO_CFG_TARGET_ENV. On a Windows host
    // cross-checking the GNU target, Cargo can expose the host env here.
    unsafe {
        std::env::set_var("CARGO_CFG_TARGET_ENV", "gnu");
    }
}

fn command_exists(command: &str) -> bool {
    std::process::Command::new(command)
        .arg("--version")
        .output()
        .is_ok()
}
