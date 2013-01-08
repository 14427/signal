// signal.rs
#[link(name = "signal", vers = "0.3", author = "sbd")];
use either::*;
use pipes::{Chan, SharedChan, Port, PortSet, Selectable, select2i, selecti};
use task::spawn;

extern mod std; // Needing this here might be a bug
mod time;

pub trait Clone {
    fn clone(&self) -> self;
}

impl<T: Copy> T: Clone {
    fn clone(&self) -> T {
        copy *self
    }
}

pub struct Signal<T: Clone Owned> {
    priv update: SharedChan< Chan<T> >,
}

impl<T: Clone Owned> Signal<T> {
    static fn new(ch: Chan<Chan<T>>) -> Signal<T> {
        Signal { update: SharedChan(ch) }
    }

    fn add_chan(&self, ch: Chan<T>) {
        self.update.send(ch)
    }

    fn lift<U: Clone Owned>(&self, f: ~fn(T) -> U) -> Signal<U> {
        lift(self, f)
    }

    fn filter(&self, default: T, f: ~fn(&T) -> bool) -> Signal<T> {
        filter(self, default, f)
    }

    fn foldp<U: Clone Owned>(&self, default: U, f: ~fn(T, U) -> U) -> Signal<U> {
        foldp(self, default, f)
    }
}

impl <T: Clone Owned> Signal<T>: Clone {
    fn clone(&self) -> Signal<T> {
        Signal { update: self.update.clone() }
    }
}

impl <T: Clone Owned> Signal<T>: Owned;

#[inline(always)]
pub fn signal_loop<T: Clone Owned, U: Clone Owned>(
    default: U,
    update: Port<T>,
    new_client: Port<Chan<U>>,
    process: ~fn(T, U) -> U,
    filter: ~fn(&T) -> bool)
{
    do spawn {
        let mut chans: ~[Chan<U>] = ~[];
        let mut value = default.clone();
        let mut open = true;
        
        loop {
            if open {
                match select2i(&update, &new_client) {
                    Left(()) => {
                        let tmp = update.recv() ;
                        if filter(&tmp) {
                            value = process(tmp, value);
                            for chans.each |c| {
                                c.send( value.clone() );
                            }
                        }
                    },
                    Right(()) => {
                        let opt_ch: Option<Chan<U>> = new_client.try_recv();
                        match opt_ch {
                            Some(ch) => {
                                ch.send( value.clone() );
                                chans.push(ch);
                            },
                            None => open = false,
                        }
                    },
                }
            } else {
                let tmp = update.recv() ;
                if filter(&tmp) {
                    value = process(tmp, value);
                    for chans.each |c| {
                        c.send( value.clone() );
                    }
                }
            }
        }
    }
}

#[inline(always)]
pub fn lift<T: Clone Owned, U: Clone Owned>(signal: &Signal<T>, f: ~fn(T) -> U) -> Signal<U> {
    let (update, chan) = pipes::stream();
    let (client_port, client_chan) = pipes::stream();

    signal.add_chan(chan);

    let initial = f( update.recv() );
    signal_loop(initial, update, client_port, |x, _| f(x), |_| true);

    Signal::new(client_chan)
}

#[inline(always)]
pub fn filter_lift<T: Clone Owned, U: Clone Owned>(signal: &Signal<T>, initial: U, filter: ~fn(&T) -> bool, process: ~fn(T, U) -> U) -> Signal<U> {
    let (update, chan) = pipes::stream();
    let (client_port, client_chan) = pipes::stream();

    signal.add_chan(chan);

    signal_loop(initial.clone(), update, client_port, process, filter);

    Signal::new(client_chan)
}

pub fn lift2<T: Clone Owned, U: Clone Owned, V: Clone Owned>(s1: &Signal<T>, s2: &Signal<U>, f: ~fn(T, U) -> V) -> Signal<V> {
    let signal = merge2(s1, s2);
    //io::println("Merged");
    let signal = signal.lift(|(x, y)| f(x, y));
    //io::println("Lifted");
    signal
}

#[inline(always)]
pub fn constant<T: Clone Owned>(value: T) -> Signal<T> {
    let (port, chan) = pipes::stream();

    do spawn {
        loop {
            let client: Chan<T> = port.recv();
            client.send( value.clone() );
        }
    }

    Signal::new(chan)
}

#[inline(always)]
pub fn dispatcher<T: Clone Owned>(default: Option<T>, f: ~fn() -> T) -> Signal<T> {
    let (client_port, client_chan) = pipes::stream();
    let (value_port, value_chan) = pipes::stream();

    do spawn {
        loop {
            let value = f();
            value_chan.send(value);
        }
    }

    let initial = match default {
        Some(value) => value,
        None => value_port.recv(),
    };

    signal_loop(initial, value_port, client_port, |x, _| x, |_| true);

    Signal::new(client_chan)
}

#[inline(always)]
pub fn merge<T: Clone Owned>(one: &Signal<T>, two: &Signal<T>) -> Signal<T> {
    let (port, chan) = pipes::stream();

    let (update1, client1) = pipes::stream();
    let (update2, client2) = pipes::stream();

    one.add_chan(client1);
    two.add_chan(client2);

    do spawn {
        let mut chans: ~[Chan<T>] = ~[];
        loop {
            while port.peek() {
                chans.push( port.recv() );
            }
            
            let value = match select2i(&update1, &update2) {
                Left(()) => update1.recv(),
                Right(()) => update2.recv(),
            };

            for chans.each |c| {
                c.send( value.clone() );
            }
        }
    }

    Signal::new(chan)
}

#[inline(always)]
pub fn merge2<T: Clone Owned, U: Clone Owned>(one: &Signal<T>, two: &Signal<U>) -> Signal<(T, U)> {
    let (port, chan) = pipes::stream();

    let (update1, client1) = pipes::stream();
    let (update2, client2) = pipes::stream();

    io::println("Before 1");
    one.add_chan(client1);
    two.add_chan(client2);
    io::println("After 1");

    do spawn {
        let mut chans: ~[Chan<(T, U)>] = ~[];
        io::println("Before 2");
        let mut last1 = update1.recv();
        let mut last2 = update2.recv();
        io::println("After 2");
        let mut push: bool;

        loop {
            push = true;
            
            match selecti([update1 as @Selectable, update2 as @Selectable, port as @Selectable]) {
                0 => last1 = update1.recv(),
                1 => last2 = update2.recv(),
                2 => {
                    let ch: Chan<(T, U)> = port.recv();
                    let value = (last1.clone(), last2.clone());
                    ch.send(value);
                    chans.push(ch);
                    push = false;
                }
                _ => fail ~"Unhandled port",
            }

            if push {
                for chans.each |c| {
                    let value = (last1.clone(), last2.clone());
                    c.send(value);
                }
            }
        }
    }

    Signal::new(chan)
}

#[inline(always)]
pub fn merges<T: Clone Owned>(signals: &[&Signal<T>]) -> Signal<T> {
    if signals.len() == 0 { fail ~"No signals provided" }    

    let (client_port, client_chan) = pipes::stream();
    let (merged_port, merged_chan) = pipes::stream();

    let mut value = None;
    let ports = do signals.map |signal| {
        let (port, chan) = pipes::stream();
        signal.add_chan(chan);
        value = port.try_recv();
        port
    };

    let port_set = PortSet { ports: ports };

    do spawn {
        loop {
            let value = port_set.recv();
            merged_chan.send(value);
        }
    }

    let value = match value {
        Some(v) => v,
        None => fail ~"No active signals provided",
    }; //value.unwrap();

    signal_loop(value, merged_port, client_port, |x, _| x, |_| true);

    Signal::new(client_chan)
}

#[inline(always)]
pub fn foldp<T: Clone Owned, U: Clone Owned>(signal: &Signal<T>, default: U, f: ~fn(T, U) -> U) -> Signal<U> {
    let (update, chan) = pipes::stream();
    let (client_port, client_chan) = pipes::stream();

    signal.add_chan(chan);

    signal_loop(default, update, client_port, f, |_| true);

    Signal::new(client_chan)
}

#[inline(always)]
pub fn filter<T: Clone Owned>(signal: &Signal<T>, default: T, f: ~fn(&T) -> bool) -> Signal<T> {
    let (update, chan) = pipes::stream();
    let (client_port, client_chan) = pipes::stream();

    signal.add_chan(chan);

    signal_loop(default, update, client_port, |x, _| x, f);

    Signal::new(client_chan)
}

pub fn count<T: Clone Owned>(signal: &Signal<T>) -> Signal<uint> {
    foldp(signal, 0 as uint, |_, x| x+1)
}

pub fn countIf<T: Clone Owned>(signal: &Signal<T>, default: T, f: ~fn(&T)-> bool) -> Signal<uint> {
    signal.filter(default, f).foldp(0 as uint, |_, x| x+1)
}

pub fn keepWhen<T: Clone Owned>(signal: &Signal<T>, other: &Signal<bool>, default: T) -> Signal<T> {
    let merged = merge2(signal, other);
    filter_lift(&merged, default, |&(_, x)| x, |(x, _), _| x)
}

pub fn split<T: Clone Owned, U: Clone Owned>(signal: &Signal<Either<T, U>>, left: T, right: U) -> (Signal<T>, Signal<U>) {
    let left = filter_lift(signal, left, is_left, |val, _| val.unwrap_left());
    let right = filter_lift(signal, right, is_right, |val, _| val.unwrap_right());
    (left, right)
}

