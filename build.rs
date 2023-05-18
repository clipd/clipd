use std::{env, fs::File, io::Write, path::Path};

use chrono::Utc;
use cmd_lib::run_fun;

fn get_build_time() -> String {
    let utc = Utc::now();
    utc.format("%F %X %z").to_string()
}

fn get_git_commit_id() -> String {
    run_fun!(git rev-parse --short HEAD).expect("Failed to get git commit id")
}

fn get_git_describe() -> String {
    run_fun!(git describe --tags --always --dirty="-dev").expect("Failed to get git describe")
}

fn get_version() -> String {
    env::var("CARGO_PKG_VERSION").expect("Failed to get CARGO_PKG_VERSION")
}

fn main() {
    add_const("BUILD_TIME", get_build_time);
    add_const("GIT_COMMIT_ID", get_git_commit_id);
    add_const("GIT_DESCRIBE", get_git_describe);
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

#[cfg(target_os = "macos")]
fn compile_res() {}
