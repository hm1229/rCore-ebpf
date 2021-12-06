use crate::arch::timer::timer_now;

pub fn post_handler() {
    debug!("after call time:{:?}", timer_now());
}
