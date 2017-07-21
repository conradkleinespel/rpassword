# Rustastic Password

[![Build Status](https://travis-ci.org/conradkdotcom/rpassword.svg?branch=master)](https://travis-ci.org/conradkdotcom/rpassword)
[![Build status](https://ci.appveyor.com/api/projects/status/6pfyblvyhaqo8lmh?svg=true)](https://ci.appveyor.com/project/conradkdotcom/rpassword)

This [Rust](http://www.rust-lang.org/) package allows you to safely read
passwords from standard input in a console application.

You can build the documentation with `cargo doc` or [view it online](https://docs.rs/rpassword/).

The source code is released under the Apache 2.0 license.

I'd appreciate feedback if you use this library :-)

## Usage

Add `rpassword` as a dependency in Cargo.toml:

```toml
[dependencies]
rpassword = "0.4"
```

Use `rpassword` within your code:

```rust
extern crate rpassword;

fn main() {
    // Prompt for a password
    let pass = rpassword::prompt_password_stdout("Password: ").unwrap();
    println!("Your password is {}", pass);
    
    // Read a password without prompt
    let pass = rpassword::read_password().unwrap();
    println!("Your password is {}", pass);
}
```

Check [examples/example.rs](examples/example.rs) for a few more examples.

## Contributors


We welcome contribution from everyone. Feel free to open an issue or a pull request at any time.

Check out the [unassigned issues](https://github.com/conradkdotcom/rpassword/issues?q=is%3Aissue+is%3Aopen+label%3Aunassigned) to get started. If you have any questions, just let us know and we'll jump in to help.

Here's a list of existing `rpassword` contributors:

* [@C4K3](https://github.com/C4K3)
* [@conradkleinespel](https://github.com/conradkleinespel)
* [@dcuddeback](https://github.com/dcuddeback)
* [@equalsraf](https://github.com/equalsraf)
* [@JanLikar](https://github.com/JanLikar)
* [@petevine](https://github.com/petevine)
* [@psych0d0g](https://github.com/psych0d0g)
* [@retep998](https://github.com/retep998)
* [@steveatinfincia](https://github.com/steveatinfincia)

Thank you very much for your help!  :smiley:  :heart:

## Donations

`rpassword` is and will remain free for everyone. If you feel like making a donation, I appreciate it though. Here are a few ways you can donate to support `rpassword` development:
- with Bitcoin: `13SPYbEZP9kvmtsKt6C9cYj4N6Ep7my6gL`
- with Litecoin: `LN6K6fn9Mq94vbiLNzW1GZ1TvWkVihyn8T`

If you cannot afford to donate, that's OK too. Just enjoy `rpassword`! :-)
