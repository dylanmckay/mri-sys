pub use self::consts::*;
pub use self::value::*;
pub use self::functions::*;
pub use self::statics::*;
pub use self::vt::*;
pub use self::ty::*;

mod value;
mod vt;
mod consts;
mod statics;
mod functions;
mod ty;
#[cfg(test)]
mod test;
#[cfg(feature = "helpers")] pub mod helpers;

extern crate libc;

#[repr(C)]
#[derive(Copy,Clone,Debug,PartialEq,Eq)]
pub struct ID(libc::uintptr_t);

#[repr(C)]
pub struct RBasic {
    flags: VALUE,
    klass: VALUE,
}

impl RBasic {
    // Value is actually a pointer to an RBasic structure.
    pub unsafe fn from_pointer(v: VALUE) -> *const Self {
        let ptr: *const RBasic = std::mem::transmute(v);
        ptr
    }
}

