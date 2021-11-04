use crate::arch::timer::timer_now;

pub fn post_handler(){
    println!("after call time:{:?}", timer_now());
}