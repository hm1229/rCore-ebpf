use crate::arch::timer::timer_now;

pub fn pre_handler() {
    debug!("before call time:{:?}", timer_now());
}
