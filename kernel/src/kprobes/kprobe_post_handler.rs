use crate::arch::timer::timer_now;

pub fn post_handler(){
    println!("after fork time:{:?}", timer_now());
}