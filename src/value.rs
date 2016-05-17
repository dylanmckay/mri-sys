use libc;
use std;

/// The inner integer of a `VALUE`.
#[allow(non_camel_case_types)]
pub type INNER_VALUE = libc::uintptr_t;

#[repr(C)]
#[derive(Copy,Clone,PartialEq,Eq)]
/// A Ruby value.
pub struct VALUE(pub INNER_VALUE);

impl std::fmt::Debug for VALUE {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "0x{:x}", self.0)
    }
}

impl std::ops::BitAnd for VALUE {
    type Output = libc::uintptr_t;

    fn bitand(self, rhs: Self) -> libc::uintptr_t {
        self.0 & rhs.0
    }
}
