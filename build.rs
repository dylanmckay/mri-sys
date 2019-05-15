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

    if should_use_flonum() {
        println!("cargo:rustc-cfg=mri_use_flonum");
    }
}

/// Logic taken from MRI's `ruby/ruby.h`.
fn should_use_flonum() -> bool {
    const SIZEOF_LONG: usize = mem::size_of::<libc::c_long>();
    const SIZEOF_LONG_LONG: usize = mem::size_of::<libc::c_longlong>();
    const SIZEOF_VOIDP: usize = mem::size_of::<*const libc::c_void>();
    const SIZEOF_DOUBLE: usize = mem::size_of::<*const libc::c_double>();

    let sizeof_value = if SIZEOF_LONG == SIZEOF_VOIDP {
        SIZEOF_LONG
    } else if SIZEOF_LONG_LONG == SIZEOF_VOIDP {
        SIZEOF_LONG_LONG
    } else {
        panic!("error: ruby requires sizeof(void*) == sizeof(long) or sizeof(LONG_LONG) to be compiled");
    };

    sizeof_value >= SIZEOF_DOUBLE
}

