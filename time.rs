extern mod std;

use std::time;
use std::time::Tm;
use std::timer;
use std::uv;

fn sleep(ms: uint) {
    let iotask = uv::global_loop::get();
    timer::sleep(iotask, ms);
}

pub fn every(ms: uint) -> Signal<Tm> {
    let initial = time::now();
    do dispatcher(Some(initial)) {
        sleep(ms);
        time::now()
    }
}

pub fn delay<T: Clone Owned>(signal: &Signal<T>, ms: uint) -> Signal<T> {
    do signal.lift |val| {
        sleep(ms);
        val
    }
}

pub fn timestamp<T: Clone Owned>(signal: &Signal<T>) -> Signal<(Tm, T)> {
    signal.lift(|val| (time::now(), val) )
}

pub fn timeOf<T: Clone Owned>(signal: &Signal<T>) -> Signal<Tm> {
    signal.lift(|_| time::now() )
}
