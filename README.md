signal
======

Functional Reactive Programming implementation for Rust

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
let clock = every(1000); // Send a signal every 1000 milliseconds

let counter = count(&clock);

counter.lift(|n| io::println("Have received %d ticks", n));
```
