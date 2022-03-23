//! A thin, slightly higher level interface to it all.
//!
//! Turn if off with `default-features = false` for this crate in your manifest.

// TODO:
//   - integer to ruby
//   - ruby to integer?
//   - line numbers passed to eval func

pub use self::protect::{catch_unwind, CaughtException};
pub use self::value::Value;

/// A binding is basically an execution context.
/// Variables and classes defined inside a binding are only
/// accessible within that binding.
#[derive(Copy, Clone, Debug)]
pub struct Binding(pub Value);

impl Binding {
    /// The top level binding (the global context) accessible in Ruby via `TOPLEVEL_BINDING`
    /// constant. This binding is a superset of all other bindings. Executions on it act
    /// as if they were executed in all possible bindings.
    pub fn top_level() -> Self {
        Binding(unsafe { std::classes::Object().constant_unprotected("TOPLEVEL_BINDING") })
    }

    /// Allocates a brand new new binding. Equivalent to `Kernel#binding`.
    pub fn allocate() -> Self {
        unsafe { Binding(std::modules::Kernel().send_unprotected("binding", &[])) }
    }
}

/// Evaluate Ruby code in a specific binding.
///
/// Rescues all Ruby exceptions.
// TODO: implement line numbers
pub fn eval(
    ruby_code: &str,
    binding: Binding,
    filename_for_debugging: Option<&str>,
) -> Result<Value, CaughtException> {
    crate::helpers::catch_unwind(|| unsafe {
        eval_unprotected(ruby_code, binding, filename_for_debugging)
    })
}

/// Evaluates Ruby code in the given binding whilst allowing Ruby unwinding
/// to unsafely propagate to Rust (causes segfaults when it hits Rust layer).
pub unsafe fn eval_unprotected(
    ruby_code: &str,
    binding: Binding,
    filename_for_debugging: Option<&str>,
) -> Value {
    let code_string = crate::helpers::to_ruby::string(ruby_code);
    let filename = filename_for_debugging.map(crate::helpers::to_ruby::string);
    // let line_number = crate::Qnil; // the argument is optional..but how to construct ruby int from rust?

    let mut argv = vec![
        *code_string,
    ];

    if let Some(filename) = filename { argv.push(*filename) };
    // if let Some(line_number) = line_number { argv.push(*line_number) };

    binding.0.send_unprotected("eval", &argv[..])
}

/// Converting Rust values to Ruby values.
pub mod to_ruby {
    use super::Value;

    /// Dereferences to a Ruby value. Makes sure the underlying data is not dropped.
    pub struct WrappedWithData<T, D> {
        value: T,
        _data: D,
    }

    /// Convert a Rust `&str` to a Ruby `String`
    pub fn string(string: &str)
        -> WrappedWithData<Value, std::ffi::CString>  {
        let cstring = std::ffi::CString::new(string).unwrap();
        let string_as_value = unsafe {
            crate::rb_str_new_cstr(cstring.as_ptr())
        };

        WrappedWithData {
            _data: cstring,
            value: Value(string_as_value),
        }
    }

    /// Convert a Rust `&str` to a Ruby `ID` / symbol
    pub fn symbol(string: &str)
        -> WrappedWithData<crate::ID, std::ffi::CString>  {
        let cstring = std::ffi::CString::new(string).unwrap();
        let string_as_value = unsafe {
            crate::rb_intern(cstring.as_ptr())
        };

        WrappedWithData {
            _data: cstring,
            value: string_as_value,
        }
    }

    impl<T, D> AsRef<T> for WrappedWithData<T, D> {
        fn as_ref(&self) -> &T { &self.value }
    }

    impl<T, D> std::ops::Deref for WrappedWithData<T, D> {
        type Target = T;

        fn deref(&self) -> &T { &self.value }
    }
}

/// Wraps `rb_protect` Ruby functionality for unwind handing.
mod protect {
    use super::{to_ruby, Value};
    use crate::VALUE;

    /// Wraps a Ruby exception `Value` and exposes its values usable in Rust.
    #[derive(Debug)]
    pub struct CaughtException {
        pub exception_object: Value,
        pub exception_class_name: String,
        pub message: String,
    }

    /// Compare the class and message of an exception against another.
    impl PartialEq for CaughtException {
        fn eq(&self, rhs: &Self) -> bool {
            let CaughtException {
                ref exception_class_name, ref message,
                exception_object: _,
            } = *self;

            *exception_class_name == rhs.exception_class_name &&
                *message == rhs.message
        }
    }

    impl Eq for CaughtException { }

    /// Wrapper over `rb_protect`, catches any Ruby exception within the given function.
    ///
    /// If you're calling into Ruby through this helper module only then you don't need this
    /// because all safe APIs run this for you. However if you're invoking raw Ruby APIs that
    /// can unwind directly, you probably do need this.
    pub fn catch_unwind<F>(
        mut f: F,
    ) -> Result<Value, CaughtException>
        where F: FnOnce() -> Value {
        let mut state: libc::c_int = 1;

        // WARNING: Don't read this after `rb_protect` is called. The `catch_unwind_internal`
        // function zeros this pointer within this callframe in place upon execution.
        // FnOnce's a bitch.
        let mut fn_ptr_buf: *mut F = &mut f;

        let fn_ptr_buf_ref: &mut *mut F = &mut fn_ptr_buf;
        let fn_ptr_buf_ptr: *mut *mut F = fn_ptr_buf_ref as _;
        let fn_ptr_buf_ptr_uint = fn_ptr_buf_ptr as usize; // sorry

        let fn_ptr_buf_ptr_as_ruby_string = to_ruby::string(&fn_ptr_buf_ptr_uint.to_string());

        let catch_unwind_internal_args = (*fn_ptr_buf_ptr_as_ruby_string).0;

        let result = unsafe {
            crate::rb_protect(catch_unwind_internal::<F>, catch_unwind_internal_args, &mut state)
        };

        if state == 0 {
            Ok(Value(result))
        } else {
            let exception_object: Value = Value(unsafe { crate::rb_errinfo() });
            unsafe { crate::rb_set_errinfo(crate::Qnil) }; // clear the exception as per guidelines

            let message = unsafe { exception_object.send_unprotected("message", &[]).to_s_unprotected() };
            let class_name = exception_object.object_class_name();

            Err(CaughtException {
                exception_object,
                exception_class_name: class_name,
                message,
            })
        }
    }

    extern "C" fn catch_unwind_internal<F>(
        fn_ptr_as_ruby_string: VALUE,
    ) -> VALUE
        where F: FnOnce() -> Value {
        let fn_ptr_buf_ptr_as_rust_string: String = unsafe {
            Value(fn_ptr_as_ruby_string).to_s_unprotected()
        };

        let fn_ptr_buf_ptr_uint: usize = match fn_ptr_buf_ptr_as_rust_string.parse() {
            Ok(uint) => uint,
            Err(..) => {
                eprintln!("this should never happen, choking on our own string");
                std::process::abort();
            },
        };
        let fn_ptr_buf_ptr: *mut *mut F = fn_ptr_buf_ptr_uint as _;
        let fn_ptr_buf_ref: &mut *mut F = unsafe { std::mem::transmute(fn_ptr_buf_ptr) };

        let fn_ptr: *mut F = std::mem::replace(fn_ptr_buf_ref, std::ptr::null_mut());
        let fn_ref: &mut F = unsafe { std::mem::transmute(fn_ptr) };
        let f = std::mem::replace(fn_ref, unsafe { std::mem::MaybeUninit::zeroed().assume_init() });

        (f)().0
    }

    /// Formats the exception like `<ClassName>: <message>`
    impl std::fmt::Display for CaughtException {
        fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(fmt, "{}: {}", self.exception_class_name, self.message)
        }
    }

    impl std::error::Error for CaughtException { }
}

mod value {
    use super::{to_ruby, CaughtException};
    use crate::VALUE;

    /// Wraps a plain old Ruby FFI `VALUE` with much more functionality.
    #[derive(Copy, Clone, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct Value(pub VALUE);

    impl Value {
        /// The Ruby `nil` value.
        pub const NIL: Self = Value(crate::Qnil);
        /// The Ruby `true` value.
        pub const TRUE: Self = Value(crate::Qtrue);
        /// The Ruby `false` value.
        pub const FALSE: Self = Value(crate::Qfalse);

        /// Sends a Ruby method and returns the result.
        pub fn send(
            &self,
            method_name: &str,
            arguments: &[Value],
        ) -> Result<Value, CaughtException> {
            crate::helpers::catch_unwind(|| unsafe {
                self.send_unprotected(method_name, arguments)
            })
        }

        /// Sends a Ruby method without rescuing Ruby unwind.
        pub unsafe fn send_unprotected(
            &self,
            method_name: &str,
            arguments: &[Value],
        ) -> Value {
            let function_symbol = to_ruby::symbol(method_name);
            let arguments = Value::convert_array(arguments);
            Value(crate::rb_funcallv(self.0, *function_symbol, arguments.len() as _, arguments.as_ptr()))
        }

        /// Gets a constant by name. Equivalent to `Object#const_get(constant_name)`.
        pub fn constant(
            &self,
            constant_name: &str,
        ) -> Result<Value, CaughtException> {
            crate::helpers::catch_unwind(|| unsafe { self.constant_unprotected(constant_name) })
        }

        /// Gets a constant by name. Equivalent to `Object#const_get(constant_name)`.
        pub unsafe fn constant_unprotected(
            &self,
            constant_name: &str,
        ) -> Value {
            let constant_symbol = to_ruby::symbol(constant_name);
            Value(crate::rb_const_get(self.0, *constant_symbol) )
        }

        /// Sets a constant. Equivalent to `Object#const_set(constant_name, value)`.
        pub fn set_constant(
            &self,
            constant_name: &str,
            value: Value,
        ) -> Result<(), CaughtException> {
            crate::helpers::catch_unwind(|| unsafe {
                self.set_constant_unprotected(constant_name, value);

                Value::NIL
            }).map(|_| ())
        }

        /// Sets a constant. Equivalent to `Object#const_set(constant_name, value)`.
        pub unsafe fn set_constant_unprotected(
            &self,
            constant_name: &str,
            value: Value,
        ) {
            let constant_symbol = to_ruby::symbol(constant_name);
            crate::rb_const_set(self.0, *constant_symbol, value.0)
        }

        /// Convert a Ruby value to a Rust string.
        ///
        /// Calls `Object#to_s` and then converts the result to a string.
        pub fn to_s(&self) -> Result<String, CaughtException> {
            super::catch_unwind(|| unsafe {
                self.send_unprotected("to_s", &[])
            }).map(|v| unsafe { v.assert_is_string_and_convert_to_string_unprotected() })
        }

        /// Convert a Ruby value to a Rust string.
        pub unsafe fn to_s_unprotected(&self) -> String {
            self.send_unprotected("to_s", &[])
                .assert_is_string_and_convert_to_string_unprotected()
        }

        unsafe fn assert_is_string_and_convert_to_string_unprotected(&self) -> String {
            let cstring_ptr = crate::rb_string_value_cstr(&self.0);
            let cstr = std::ffi::CStr::from_ptr(cstring_ptr);

            cstr.to_str().expect("invalid UTF-8").to_owned()
        }

        /// Convert a Ruby string to a Rust string.
        /// Gets the class name for an object
        pub fn object_class_name(&self) -> String {
            unsafe {
                let cstr_ptr = crate::rb_obj_classname(self.0);
                std::ffi::CStr::from_ptr(cstr_ptr).to_str().unwrap().to_owned()
            }
        }

        /// Calls `Object#inspect`
        pub fn inspect(&self) -> Result<Value, CaughtException> {
            super::catch_unwind(|| unsafe { self.inspect_unprotected() })
        }

        /// Calls `Object#inspect`
        pub unsafe fn inspect_unprotected(&self) -> Value {
            self.send_unprotected("inspect", &[])
        }

        /// Checks if this object is of the given value type.
        pub fn is_of_value_type(&self, value_type: crate::value_type) -> bool {
            crate::TYPE_P(self.0, value_type)
        }

        /// Checks if this value is `nil`.
        pub fn is_nil(&self) -> bool { self.0 == crate::Qnil }


        pub fn convert_array(values: &[Value]) -> &[VALUE] {
            unsafe { std::mem::transmute(values) } // safe because of #[repr(transparent)]
        }
    }

    impl std::fmt::Display for Value {
        fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
            self.to_s().unwrap_or_else(|e| format!("ERROR: unexpected ruby exception: {}", e)).fmt(fmt)
        }
    }

    impl std::fmt::Debug for Value {
        fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
            super::catch_unwind(|| unsafe {
                let inspect_string = self.inspect_unprotected().to_s_unprotected();
                write!(fmt, "{}", inspect_string).ok();

                Value::NIL
            }).expect("Ruby method #inspect failed");

            Ok(())
        }
    }

    impl std::str::FromStr for Value {
        type Err = CaughtException;

        fn from_str(s: &str) -> Result<Self, CaughtException> {
            Ok(*to_ruby::string(s))
        }
    }

    impl From<bool> for Value {
        fn from(b: bool) -> Self {
            if b { Value::TRUE } else { Value::FALSE }
        }
    }

    impl From<VALUE> for Value {
        fn from(v: VALUE) -> Self { Value(v) }
    }

    impl Into<VALUE> for Value {
        fn into(self) -> VALUE { self.0 }
    }

    impl AsRef<VALUE> for Value {
        fn as_ref(&self) -> &VALUE { &self.0 }
    }

    impl std::ops::Deref for Value {
        type Target = VALUE;

        fn deref(&self) -> &VALUE { &self.0 }
    }

    impl std::ops::DerefMut for Value {
        fn deref_mut(&mut self) -> &mut VALUE { &mut self.0 }
    }
}

/// Get the builtin global/static class/module `Value` instances like `Kernel`, `Object`,
/// `Integer`, etc.
pub mod std {
    #![allow(non_snake_case)]

    pub mod modules {
        use super::super::Value;

        pub fn Kernel() -> Value { Value(unsafe { crate::rb_mKernel }) }
        pub fn Math() -> Value { Value(unsafe { crate::rb_mMath }) }
    }

    pub mod classes {
        use super::super::Value;

        pub fn Object() -> Value { Value(unsafe { crate::rb_cObject}) }
        pub fn Array() -> Value { Value(unsafe { crate::rb_cArray}) }
        pub fn Binding() -> Value { Value(unsafe { crate::rb_cBinding}) }
        pub fn Class() -> Value { Value(unsafe { crate::rb_cClass}) }
        pub fn Module() -> Value { Value(unsafe { crate::rb_cModule}) }
        pub fn NilClass() -> Value { Value(unsafe { crate::rb_cNilClass}) }
        pub fn Integer() -> Value { Value(unsafe { crate::rb_cInteger}) }
        pub fn Hash() -> Value { Value(unsafe { crate::rb_cHash}) }
        pub fn Float() -> Value { Value(unsafe { crate::rb_cFloat}) }
    }
}
