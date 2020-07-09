# RustMS
RustMS is an attempt at implementing the Maplestory server end from scratch in Rust.

At the time at which I am writing this, I know very little about Rust, and I know very little about the actual implementation of the Maplestory server end. I am however quite interested in Rust as I've always had a superficial interest in lower level languages such as C but never really invested enough time to actually get comfortable using them. I've also had recent thoughts about trying to implement a Maplestory server backend due to the fond memories I have with this game from my childhood, and because I am aware that there are several open source implementations floating around Github. My main inspiration at the time being is [Hucaru's project Valhalla](https://github.com/Hucaru/Valhalla), which at the time being is a WIP implementation of a v28 server end in Go.

# Roadmap

## Task 1: Sending Packets to the client

### Modelling a packet builder
Our first task will be to create a model that allows us to build packets that we send from the server to the client.

There are a number of different data types that we may wish to write to a packet... From some preliminary research, the primary types we want to support are...
* i8
* i8[]
* i16
* i32
* i64
* ASCII String (As an array of i8)
* ASCII String (As an array of i8 with an i16 header denoting length)
* ASCII String (As an array of i8 plus a null terminator)

It also appears that all integer values are read by the client in *little endian* byte order, hence we will need to account for this when building our packet writer. 

We will define a structure that builds up a byte vector and has methods for writing *little endian* integers, as well as ASCII strings to the vector. We should then be able to retrieve a slice of the vector and write that to the socket.

### Properly implementing the handshake
The communication between the client and the server is meant to be encrypted using some variant of AES. The initial 'hello' handshake between the client and the server generates the keys for communication between a particular client and the server. Currently, we've hardcoded this handshake, however it would be a good idea to properly implement this moving forward.

More to be said for this subtask soon... Will need to read into AES and the handshake.