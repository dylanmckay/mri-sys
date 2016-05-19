use std::mem;

extern crate libc;

#[cfg(target_os = "linux")]
const LINK_CRYPT: bool = true;
#[cfg(not(target_os = "linux"))]
const LINK_CRYPT: bool = false;

fn main() {
    let ruby_lib_name = match std::env::var("RUBY_LIB") {
        Ok(lib) => lib,
        Err(..) => "ruby".to_owned(),
    };

    println!("cargo:rustc-link-lib=dylib={}", ruby_lib_name);

    if LINK_CRYPT { println!("cargo:rustc-link-lib=dylib=crypt"); }
    println!("cargo:rustc-link-lib=dylib=gmp");

    if mem::size_of::<libc::uintptr_t>() >= mem::size_of::<f64>() {
        println!("cargo:rustc-cfg=mri_use_flonum");
    }
}

