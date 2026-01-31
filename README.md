# flights
flights is a Rust project designed for real-time analysis, visualization of aircraft being tracked on the Open Glider Network. It uses data published by their APRS servers (see [here](http://wiki.glidernet.org/aprs-interaction-examples)),
and this project creates TCP stream to read, parse, and handle downstream calculations/rendering on a multithreaded system.

## purpose
This is a personal project to dive into the Rust programming language, with a particular focus on concepts like multithreading and real-time systems to build efficient and responsive applications for processing live data.

## Installation
Follow instructions [here](https://rust-lang.org/learn/get-started/) to install a Rust development environment.

### Tests
Run tests with `cargo test`

### Running the gui
A config toml file (for configuring glidernet APRS server). An example config file (`connection.toml.example`) is provided.

To run the gui, simply run the application with the following command:
`cargo run -- --logging-level info --gui --config-file <path to your config file>`

To run for a specified duration, add the `--duration` flag.

Full list of flags are shown using the `-h` flag:
`cargo run -- -h`
