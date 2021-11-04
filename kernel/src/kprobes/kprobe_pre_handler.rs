use crate::arch::timer::timer_now;

pub fn pre_handler(){
    println!("before call time:{:?}", timer_now());
}