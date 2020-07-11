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

It also appears that all integer values are read by the client in *little endian* byte order, hence we will need to account for this when building our packet writer. 

We will define a structure that builds up a byte vector and has methods for writing *little endian* integers, as well as ASCII strings and length headered ASCII Strings to the vector. We will then be able to grab the packet data stored in this struct as a slice, and write that directly to a socket.

## Task 2: Implementing Maplestory's custom AES encryption
Older versions of Maplestory, such as this one, use a customized version of AES for its client-server communication. The initial 'hello' handshake between the client and the server establishes the keys for communication between a particular client and the server by including a pair of initialization vectors. Currently, we've hardcoded this handshake, and we're not actually generating the keys.

A neat implementation of this customized algorithm can be found [here](https://github.com/hadeutscher/MapleLib/tree/master/MapleCryptoLib). For this project, we will port this implementation over to Rust.


## Task 3: Model a client session
Now that we've implemented a way by which we write packets to our client, as well as a way of generating encryption keys, it becomes necessary to model the server's open connections with clients (how else would we keep track of which keys to use for which connection?). For now, would be sufficient to define a session struct that stores the encryption keys and `TcpStream`, and defines methods to send packets to the client. 

This session struct will grow in size as the project moves forward to track things about the client connection, and will likely have to be segmented into smaller parts to separate responsibilities. For now, we'll keep things simple.