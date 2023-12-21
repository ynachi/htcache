# DESIGN

This document describes the design of gcache server. It is a couple of notes about the organization of the code which
can improve its understanding and the reasoning behind some decisions.
We intentionally try
to use the standard lib as much as possible
to learn in details what Rust has to offer before jumping to high-level crates that hide these details from us.

## RESP (Redis framing protocol)
This is a client server application. Which means there is a need to transfer data over a network protocol. Data
traversing networks need to be serialized and deserialized at reception. It is an agreement between the client and the
server about the meanings of the data exchanged. We decided to implement the [RESP](https://redis.io/docs/reference/protocol-spec/) protocol, which is a simple, yet
powerful serialization protocol. The second reason we chose this protocol is that we are building a kind of Redis
clone and wanted existing Redis clients to be compatible with our server.

## Code organization

### Thread Pool
The [Thread Pool](src/threadpool.rs) implements the tread pool pattern
which allows a finite number of threads to process a large number of jobs.
We rely on a Rust MPSC channel.
This channel construct only allows a single consumer while in our case, we need to share it to many threads.
The reason is that there we expect these threads to all consume jobs sent through the channel.
So we augmented MPSC to allow many consumers on the channel.
It is worth mentioning that dropping (intentionally or not) a Pool will also drop all the threads so no job will be processed in this case.
Why thread pool? Threads are limited and expensive resource, and it is easy to DDOS if not limited.

### Frame module
The [frame](src/frame.rs) module implements the RESP protocol. Not everything is implemented for now.
Also, we rely on version 3 at this time.
Frames are defined as Rust Enum variant like below which eases the use of pattern matching to encode/decode a Frame: 
```Rust
pub(crate) enum Frame {
    Simple(String),
    Error(String),
    Integer(i64),
    Bulk(String),
    Array(Vec<Frame>),
    Null,
    Boolean(bool),
}
```

### Error module
The [Error](src/error.rs): The error module defines custom errors for frame encoding/decoding.

### Command module
The command module is organized in submodules, each of them representing a command.
Every command should implement the [Command trait](src/cmd/mod.rs).
Adding a new command is a two-step process.
First, one needs to add a new implementation of the trait as a `cmd` submodule.
Second, you need to update the factory method `apply_command` in the [connection] module.
While the connection method could become too big in the long run,
it is an acceptable trade-off for now to avoid using dynamic dispatch (dyn).
