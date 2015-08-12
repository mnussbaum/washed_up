`washed_up` is a toy actor library for Rust.

[![Build Status](https://travis-ci.org/mnussbaum/washed_up.svg)](https://travis-ci.org/mnussbaum/washed_up)

This actor library provides:
  * Light weight "green" threads
  * Message passing semantics

It hopes to provide in the future:
  * Integrated non-blocking IO (possibly via [mioco](https://github.com/dpc/mioco))
  * Easier to use scheduling
  * Distributed actors and message passing

`washed_up` uses [coroutines](https://github.com/rustcc/coroutine-rs) to
provide light weight concurrency. To effectively use coroutines use
non-blocking code.  Since coroutines are cooperatively scheduled it's up to
your code to relinquish control back to the scheduler when it has reached a
good stopping point. Typically, this might mean having your actor code invoke
the scheduler after processing each message.

Here's an example:

```rust
extern crate coroutine;
extern crate rustc_serialize;
extern crate uuid;
extern crate washed_up;

use coroutine::Coroutine;
use rustc_serialize::json::Json;
use uuid::Uuid;
use washed_up::Supervisor;

fn main() {
    // The supervisor coordinates our actors
    let supervisor = Supervisor::new("People");

    // Actors are identified with pids and have names primarily for debugging
    let pid: Uuid = supervisor.spawn(
        "Fred",
        |receiver| {
            loop {
                let message = receiver.recv().unwrap();
                if message.is_null() {
                    break;
                }
                println!("Received: {}", message);

                // This yields control to the scheduler so other actors can run.
                // Execution will resume right where it left off
                Coroutine::sched();
            }
            ()
        }
    ).unwrap();

    // Messages are sent with pids
    supervisor.send_message(pid, Json::from_str("{\"hello\": \"hi\"}").unwrap()).unwrap();

    // Our actor body is set to quit when it sees null
    supervisor.send_message(pid, Json::from_str("null").unwrap()).unwrap();

    // This will block until the actor body has finished
    supervisor.join(pid).unwrap()
}
```

Your actor body function must have the following signature:
```rust
Fn(std::sync::mpsc::Receiver<Json>) -> ()
```

Dependencies you will probably need to pull in:
* [coroutine](https://github.com/rustcc/coroutine-rs)
* [uuid](https://github.com/rust-lang/uuid)
* [rustc-serialize](https://github.com/rust-lang/rustc-serialize)

The coroutine library requires Rust nightly, so this library does as well.
