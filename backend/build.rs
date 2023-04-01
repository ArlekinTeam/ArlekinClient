use std::{env, fs, path::Path, process::Command};

use fs_extra::{copy_items, dir};

fn main() {
    println!("cargo:rerun-if-changed=../");

    let out_dir = env::var("OUT_DIR").unwrap();
    let mut home = Path::new(out_dir.as_str());

    while !home.ends_with("backend") {
        home = home.parent().unwrap();
    }

    let frontend = home.parent().unwrap().join(Path::new("frontend"));
    println!("{}", frontend.display());

    let mut command = Command::new("trunk");
    command.arg("build").current_dir(frontend.clone());
    if env::var("PROFILE").unwrap() == "release" {
        command.arg("--release");
    }
    command.spawn().expect("Trunk command failed.");

    let dist = home.join(Path::new("dist"));
    fs::create_dir_all(dist.clone()).expect("Unable to create dist directory.");

    let options = dir::CopyOptions::new().overwrite(true);
    copy_items(&[frontend.join(Path::new("static"))], dist, &options)
        .expect("Unable to copy static files.");
    copy_items(&[frontend.join(Path::new("dist"))], home, &options)
        .expect("Unable to copy dist files.");
}
