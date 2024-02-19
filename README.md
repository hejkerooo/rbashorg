# RBashOrg

Simple joke randomizer from [bash.org.pl](http://bash.org.pl/) made as CLI Tool in Rust.

## Description
Main motivation of this project was to learn basic concepts about UTF-8, operating on bytes, vectors manipulation, storing data.  
The core feature of it is minimal memory usage. It benefits from making simple indexes of each joke (start and end position), reading each joke in form of chunks.  

## Future improvements
- [x] Improve UTF-8 decode error handling
- [x] Implement better handling of UTF-8 2/3/4 byte characters
- [] Unit tests
- [] Make it a proper CLI with help, random etc. commands
- [] Remembering read joke
- [x] Improve memory usage even further (right now it's ~900kb)
- [x] Change random algorithm to not load all indexes but rather select random line and find closest `%` character to not store all indexes
- [x] Get rid of `reqwest` in favour of `TcpStream`

## Cache storage
All jokes are downloaded from http://bash.org.pl/text
They are being cached in [data local dir](https://docs.rs/dirs/latest/dirs/fn.data_local_dir.html)
i.e. `C:\Users\hejkerooo\AppData\Local\RBashOrg\RBashOrg\data`

## Getting Started

### Dependencies

* Rust 1.56 or later
* Compatible with Windows, Linux, and MacOS

### Installing

How to install the project, step by step.

```bash
# Build the project (debug mode)
cargo build

# Or build the project (release mode)
cargo build --release