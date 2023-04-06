use std::{env, fs::File, io::Write, path::Path, process::Command};

use chrono::Utc;

fn get_build_time() -> String {
    let utc = Utc::now();
    utc.format("%F %X %z").to_string()
}

fn get_git_commit_id() -> String {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .expect("Failed to read stdout");
    String::from_utf8(output.stdout).expect("Failed to read stdout")
}
fn get_version() -> String {
    env::var("CARGO_PKG_VERSION").expect("Failed to get CARGO_PKG_VERSION")
}

fn main() {
    add_const("BUILD_TIME", get_build_time);
    add_const("GIT_COMMIT_ID", get_git_commit_id);
    add_const("VERSION", get_version);
    compile_res();
}

fn add_const(file_name: &'static str, value: fn() -> String) {
    let out_dir_path = env::var("OUT_DIR").expect("Failed to get OUT_DIR");
    let out_dir = Path::new(&out_dir_path);
    write_to(&out_dir.clone().join(file_name), &value().trim().as_bytes());
}

fn write_to(path: &Path, bytes: &[u8]) {
    let mut f = File::create(path).expect("Failed to create file");
    f.write_all(bytes).expect("Failed to write file");
}

#[cfg(target_os = "windows")]
fn compile_res() {
    let mut res = winres::WindowsResource::new();
    res.set_icon_with_id("assets/running.ico", "ic_01_running");
    res.set_icon_with_id("assets/paused.ico", "ic_02_paused");
    res.compile().unwrap();
}

#[cfg(target_os = "linux")]
fn compile_res() {
    println!("cargo:rustc-link-lib=dylib=X11");
    println!("cargo:rustc-link-lib=dylib=Xfixes");
}

#[cfg(target_os = "maco")]
fn compile_res() {}
