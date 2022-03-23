use super::*;
use crate::helpers::*;

pub fn eval(s: &str) -> Result<Value, CaughtException> {
    crate::helpers::eval(s, Binding::top_level(), None)
}

#[test]
pub fn vm_can_eval_stuff() {
    unsafe {
        ruby_init();

        assert_eq!(eval("nil"), Ok(Value::NIL));

        let number_ten = eval("10").unwrap();
        let five_plus_five = eval("5+5").unwrap();

        assert!(eval("nil").unwrap().is_nil());
        assert!(eval("1").unwrap().is_of_value_type(T_FIXNUM));
        assert!(five_plus_five.is_of_value_type(T_FIXNUM));
        assert!(eval("1.0").unwrap().is_of_value_type(T_FLOAT));
        assert_eq!(eval("true || false").unwrap(), Value::TRUE);
        assert_eq!(eval("true && false").unwrap(), Value::FALSE);

        assert_eq!(number_ten.to_s().unwrap(), "10".to_owned());
        assert_eq!(five_plus_five.to_s().unwrap(), "10".to_owned());
        assert_eq!(number_ten, five_plus_five);

        ruby_cleanup(0);
    }
}
