use std::ffi::{CString, CStr};
use std::io::Write;

#[derive(Copy, Clone)]
pub struct Value(mri_sys::VALUE);

impl Value {
    pub fn send(&self, method: &str) -> Value {
        let method_string = CString::new(method).unwrap();
        let interned_method_name = unsafe {
            mri_sys::rb_intern(method_string.as_ptr())
        };

        let result = unsafe { mri_sys::rb_funcall(self.0, interned_method_name, 0) };
        Value(result)
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let c_str = unsafe { mri_sys::rb_string_value_cstr(&self.0) };
        let c_str = unsafe { CStr::from_ptr(c_str) };
        write!(fmt, "{}", c_str.to_str().unwrap())
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{}", self.send("inspect"))
    }
}

fn eval(ruby_code: &str) -> Result<Value, Value> {
    let mut state = 0;

    let code_string = CString::new(ruby_code).expect("input code is not a valid C string");

    let result = unsafe {
        mri_sys::rb_eval_string_protect(code_string.as_ptr(), &mut state as *mut _)
    };

    if state == 0 {
        Ok(Value(result))
    } else {
        let exception_value = unsafe { mri_sys::rb_errinfo() };
        Err(Value(exception_value))
    }
}

fn main() {
    unsafe { mri_sys::ruby_init() };

    loop {
        print!("cool-interpreter:8=====D -- ");
        std::io::stdout().flush().unwrap();

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).expect("could not read from stdin");

        match &input.trim().to_lowercase()[..] {
            "exit" | "quit" => break,
            _ => match eval(&input) {
                Ok(value) => println!("-> {:?}", value),
                Err(e) => eprintln!("ERROR, EXCEPTION RAISED: {:?}", e),
            },
        }
    }

    unsafe { mri_sys::ruby_cleanup(0) };
}
