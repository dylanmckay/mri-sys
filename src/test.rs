use super::*;
use std::ffi::CString;
use libc;

pub fn c_str(s: &str) -> CString {
    CString::new(s).unwrap()
}

pub fn eval(s: &str) -> (libc::c_int, VALUE) {
    let mut state: libc::c_int = 0;

    let result = unsafe {
        rb_eval_string_protect(c_str(s).as_ptr(), &mut state)
    };

    (state, result)
}

#[test]
pub fn vm_can_eval_stuff() {
    unsafe {
        ruby_init();

        let number_ten = eval("10").1;
        let five_plus_five = eval("5+5").1;

        assert_eq!(eval("nil"), (0, Qnil));
        assert!(TYPE_P(eval("1").1, T_FIXNUM));
        assert!(TYPE_P(eval("1.0").1, T_FLOAT));
        assert_eq!(eval("Fixnum").1, rb_cFixnum);
        assert_eq!(eval("Fixnum.class").1, rb_cClass);
        assert_eq!(eval("Fixnum.class.class").1, rb_cClass);
        assert!(TYPE_P(eval("false || true").1, T_TRUE));
        assert!(TYPE_P(eval("true || false").1, T_TRUE));
        assert!(TYPE_P(eval("false || 5").1, T_FIXNUM));

        assert_eq!(number_ten, five_plus_five);

        ruby_cleanup(0);
    }
}
