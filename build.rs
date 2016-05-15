use std::mem;

extern crate libc;

fn main() {
    let ruby_lib_name = match std::env::var("RUBY_LIB") {
        Ok(lib) => lib,
        Err(..) => "ruby".to_owned(),
    };

    println!("cargo:rustc-link-lib=dylib={}", ruby_lib_name);

    if mem::size_of::<libc::uintptr_t>() >= mem::size_of::<f64>() {
        println!("cargo:rustc-cfg=mri_use_flonum");
    }
}

