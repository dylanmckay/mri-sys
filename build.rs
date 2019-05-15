use std::mem;
use std::process::Command;

extern crate libc;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord)]
struct RubyVersion(pub u32, pub u32);

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

    ruby_version_supports_flonum() &&
        sizeof_value >= SIZEOF_DOUBLE
}

fn ruby_version_supports_flonum() -> bool {
    const FIRST_RUBY_VERSION_WITH_FLONUM: RubyVersion = RubyVersion(2, 0);

    current_ruby_version().map(|current_version| current_version >= FIRST_RUBY_VERSION_WITH_FLONUM)
        .unwrap_or(true) // Optimistically assume the current Ruby version is >= 2.0.0.
}

fn current_ruby_version() -> Option<RubyVersion>  {
    let output = Command::new("ruby")
        .args(&["--version"])
        .output()
        .ok();

    output.map(|version_output| {
        let version_line = String::from_utf8(version_output.stdout).unwrap().trim().to_owned();
        let mut version_line_parts = version_line.split_whitespace();

        assert_eq!(Some("ruby"), version_line_parts.next(), "expected version string to start with 'ruby'");
        let version_str = version_line_parts.next().expect("expected ruby --version to contain a version");
        let mut version_parts = version_str.split(".");

        let major_version = version_parts.next().unwrap().parse().expect("expected Ruby major version to be an integer");
        let minor_version = version_parts.next().unwrap().parse().expect("expected Ruby minor version to be an integer");

        RubyVersion(major_version, minor_version)
    })
}

impl std::cmp::PartialOrd for RubyVersion {
    fn partial_cmp(&self, rhs: &Self) -> Option<std::cmp::Ordering> {
        if self.0 > rhs.0 {
            Some(std::cmp::Ordering::Greater)
        } else if rhs.0 > self.0 {
            Some(std::cmp::Ordering::Less)
        } else {
            assert_eq!(self.0, rhs.0); // same major.

            self.1.partial_cmp(&rhs.1)
        }
    }
}

