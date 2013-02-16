signal
======

Functional Reactive Programming implementation for [Rust](http://www.rust-lang.org/)

Currently a WIP

Inspired by [Elm](http://elm-lang.org/)

Examples
--------

Hello World:
```
let hello = constant(~"Hello world");
hello.lift(|msg| io::println(msg) );
```

Clock:
```
let clock = &every(1000); // Send a signal every 1000 milliseconds

let counter = count(clock);

counter.lift(|n| io::println("Have received %d ticks", n));
```

Concurrency:
```
let (a, b, c, d) = ( constant(10), constant(20), constant(30), constant(40) );
    
let s1 = a.lift(|x| x*2);
let s2 = b.lift(|x| x*2);
let s3 = lift2(&c, &d, |a, b| a+b);

lift3(&s1, &s2, &s3, |x, y, z| io::println(fmt!("%d + %d + %d = %d", x, y, z, x+y+z)));
```
