// signal.rs
#[link(name = "signal", vers = "0.3", author = "sbd")];
use either::*;
use pipes::*;
use task::spawn;

extern mod std; // Needing this here might be a bug
pub mod time;

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

/*impl <T: Clone Owned> Signal<T>: Clone {
    fn clone(&self) -> Signal<T> {
        Signal { update: self.update.clone() }
    }
}*/

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
        let mut update_open = true;
        let mut client_open = true;
        
        loop {
            if client_open && update_open {
                match select2i(&update, &new_client) {
                    Left(()) => {
                        let opt_tmp = update.try_recv();
                        match opt_tmp {
                            Some(tmp) => {
                                if filter(&tmp) {
                                    value = process(tmp, value);
                                    for chans.each |c| {
                                        c.send( value.clone() );
                                    }
                                }
                            },
                            None => update_open = false,
                        }
                    },
                    Right(()) => {
                        let opt_ch: Option<Chan<U>> = new_client.try_recv();
                        match opt_ch {
                            Some(ch) => {
                                ch.send( value.clone() );
                                chans.push(ch);
                            },
                            None => client_open = false,
                        }
                    },
                }
            } else if update_open {
                let opt_tmp = update.try_recv();
                match opt_tmp {
                    Some(tmp) => {
                        if filter(&tmp) {
                            value = process(tmp, value);
                            for chans.each |c| {
                                c.send( value.clone() );
                            }
                        }
                    },
                    None => update_open = false,
                }
            } else if client_open {
                let opt_ch: Option<Chan<U>> = new_client.try_recv();
                match opt_ch {
                    Some(ch) => {
                        ch.send( value.clone() );
                        chans.push(ch);
                    },
                    None => client_open = false,
                }
            } else {
                break
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
    let signal = signal.lift(|(x, y)| f(x, y));
    signal
}

pub fn lift3<A: Clone Owned, B: Clone Owned, C: Clone Owned, V: Clone Owned>(
    s1: &Signal<A>, s2: &Signal<B>, s3: &Signal<C>, f: ~fn(A, B, C) -> V) -> Signal<V>
{
    merge3(s1, s2, s3).lift(|(x, y, z)| f(x, y, z))
}

pub fn lift4<A: Clone Owned, B: Clone Owned, C: Clone Owned, D: Clone Owned, V: Clone Owned>(
    s1: &Signal<A>, s2: &Signal<B>, s3: &Signal<C>, s4: &Signal<D>, f: ~fn(A, B, C, D) -> V) -> Signal<V>
{
    let m1 = &merge2(s1, s2);
    let m2 = &merge2(s3, s4);
    merge2(m1, m2).lift(|((a, b), (c, d))| f(a, b, c, d))
}

pub fn lift5<A: Clone Owned, B: Clone Owned, C: Clone Owned, D: Clone Owned, E: Clone Owned, V: Clone Owned>(
    s1: &Signal<A>, s2: &Signal<B>, s3: &Signal<C>, s4: &Signal<D>, s5: &Signal<E>, f: ~fn(A, B, C, D, E) -> V) -> Signal<V>
{
    let m1 = &merge2(s1, s2);
    let m2 = &merge3(s3, s4, s5);
    merge2(m1, m2).lift(|((a, b), (c, d, e))| f(a, b, c, d, e))
}

pub fn lift6<A: Clone Owned, B: Clone Owned, C: Clone Owned, D: Clone Owned, E: Clone Owned, F: Clone Owned, V: Clone Owned>(
    s1: &Signal<A>, s2: &Signal<B>, s3: &Signal<C>, s4: &Signal<D>, s5: &Signal<E>, s6: &Signal<F>, op: ~fn(A, B, C, D, E, F) -> V) -> Signal<V>
{
    let m1 = &merge3(s1, s2, s3);
    let m2 = &merge3(s4, s5, s6);
    merge2(m1, m2).lift(|((a, b, c), (d, e, f))| op(a, b, c, d, e, f))
}

#[inline(always)]
pub fn constant<T: Clone Owned>(value: T) -> Signal<T> {
    let (port, chan) = pipes::stream();

    do spawn {
        loop {
            let client: Option<Chan<T>> = port.try_recv();
            match client {
                Some(ch) => ch.send( value.clone() ),
                None => break,
            }
        }
    }

    Signal::new(chan)
}

#[inline(always)]
pub fn dispatcher<T: Clone Owned>(default: Option<T>, f: ~fn() -> Option<T>) -> Signal<T> {
    let (client_port, client_chan) = pipes::stream();
    let (value_port, value_chan) = pipes::stream();

    do spawn {
        loop {
            match f() {
                Some(value) => value_chan.send(value),
                None => break,
            }
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
            // FIXME: Add to select
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

    one.add_chan(client1);
    two.add_chan(client2);

    do spawn {
        let mut chans: ~[Chan<(T, U)>] = ~[];

        let mut last1 = update1.recv();
        let mut last2 = update2.recv();

        let mut push: bool;

        let mut u1_open = true;
        let mut u2_open = true;
        let mut client_open = true;

        let header1 = PacketHeader();
        let header2 = PacketHeader();
        let header3 = PacketHeader();

        let mut ports = ~[update1.header(), update2.header(), port.header()];

        while u1_open || u2_open || client_open {
            push = false;
            match selecti( ports ) {
                0 => {
                    match update1.try_recv() {
                        Some(value) => {
                            last1 = value;
                            ports[0] = update1.header();
                            push = true;
                        }
                        None => {
                            u1_open = false;
                            ports[0] = &header1;
                        }
                    }
                }
                1 => {
                    match update2.try_recv() {
                        Some(value) => {
                            last2 = value;
                            ports[1] = update2.header();
                            push = true;
                        }
                        None => {
                            u2_open = false;
                            ports[1] = &header2;
                        }
                    }
                }
                2 => {
                    match port.try_recv() {
                        Some(ch) => {
                            let ch: Chan<(T, U)> = ch;
                            let value = (last1.clone(), last2.clone());
                            ch.send( value );
                            chans.push( ch );
                            ports[2] = port.header();
                        }
                        None => {
                            client_open = false;
                            ports[2] = &header3;
                        }
                    }
                }
                _ => fail ~"Merge incorrectly implemented",
            }

            if push {
                for chans.each |ch| {
                    let value = (last1.clone(), last2.clone());
                    if !ch.try_send( value ) {
                        fail ~"This need to get fixed";
                    }
                }
            }
        }
    }

    Signal::new(chan)
}

#[inline(always)]
pub fn merge3<A: Clone Owned, B: Clone Owned, C: Clone Owned>(
    one: &Signal<A>,
    two: &Signal<B>,
    three: &Signal<C>
) -> Signal<(A, B, C)> {
    let (port, chan) = pipes::stream();

    let (update1, client1) = pipes::stream();
    let (update2, client2) = pipes::stream();
    let (update3, client3) = pipes::stream();

    one.add_chan(client1);
    two.add_chan(client2);
    three.add_chan(client3);

    do spawn {
        let mut chans: ~[Chan<(A, B, C)>] = ~[];

        let mut last1 = update1.recv();
        let mut last2 = update2.recv();
        let mut last3 = update3.recv();

        let mut push: bool;

        let mut u1_open = true;
        let mut u2_open = true;
        let mut u3_open = true;
        let mut client_open = true;

        let header1 = PacketHeader();
        let header2 = PacketHeader();
        let header3 = PacketHeader();
        let header4 = PacketHeader();

        let mut ports = ~[update1.header(), update2.header(), update3.header(), port.header()];

        while u1_open || u2_open || u3_open || client_open {
            push = false;
            match selecti( ports ) {
                0 => {
                    match update1.try_recv() {
                        Some(value) => {
                            last1 = value;
                            ports[0] = update1.header();
                            push = true;
                        }
                        None => {
                            u1_open = false;
                            ports[0] = &header1;
                        }
                    }
                }
                1 => {
                    match update2.try_recv() {
                        Some(value) => {
                            last2 = value;
                            ports[1] = update2.header();
                            push = true;
                        }
                        None => {
                            u2_open = false;
                            ports[1] = &header2;
                        }
                    }
                }
                2 => {
                    match update3.try_recv() {
                        Some(value) => {
                            last3 = value;
                            ports[2] = update3.header();
                            push = true;
                        }
                        None => {
                            u3_open = false;
                            ports[2] = &header3;
                        }
                    }
                }
                3 => {
                    match port.try_recv() {
                        Some(ch) => {
                            let ch: Chan<(A, B, C)> = ch;
                            let value = (last1.clone(), last2.clone(), last3.clone());
                            ch.send( value );
                            chans.push( ch );
                            ports[3] = port.header();
                        }
                        None => {
                            client_open = false;
                            ports[3] = &header4;
                        }
                    }
                }
                _ => fail ~"Merge incorrectly implemented",
            }

            if push {
                for chans.each |ch| {
                    let value = (last1.clone(), last2.clone(), last3.clone());
                    if !ch.try_send( value ) {
                        fail ~"This need to get fixed";
                    }
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
    };

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

