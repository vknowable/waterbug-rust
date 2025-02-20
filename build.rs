use std::{
  env,
  fs,
  path::{Path, PathBuf},
};

fn main() {
  if env::var("CARGO_CFG_TARGET_OS").unwrap() == "android" {
      android();
  }
}

fn android() {
  println!("cargo:rustc-link-lib=c++_shared");

  // Paths for libc++_shared.so
  let output_path = match env::var("CARGO_NDK_OUTPUT_PATH") {
      Ok(path) => PathBuf::from(path),
      Err(_) => return, // Exit if the output path isn't available
  };

  let target_dir = output_path.join(&env::var("CARGO_NDK_ANDROID_TARGET").unwrap());
  fs::create_dir_all(&target_dir).ok(); // Ensure target directory exists

  let sysroot_libs_path = PathBuf::from(env::var_os("CARGO_NDK_SYSROOT_LIBS_PATH").unwrap());
  let lib_path = sysroot_libs_path.join("libc++_shared.so");

  // Copy libc++_shared.so to the target directory
  if let Err(e) = fs::copy(&lib_path, target_dir.join("libc++_shared.so")) {
      eprintln!("Failed to copy libc++_shared.so: {}", e);
  }
}