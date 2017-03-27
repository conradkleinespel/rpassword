# Rustastic Password

[![Build Status](https://travis-ci.org/conradkleinespel/rustastic-password.svg?branch=master)](https://travis-ci.org/conradkleinespel/rustastic-password)
[![Build status](https://ci.appveyor.com/api/projects/status/812odw3tw6oec5sw/branch/master?svg=true)](https://ci.appveyor.com/project/conradkleinespel/rustastic-password/branch/master)

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

Import the `rpassword` crate and use the `promt_password_stdout()` to show a message on `stdout` and read a password into a `String`:

```rust
extern crate rpassword;

fn main() {
    let pass = rpassword::prompt_password_stdout("Password: ").unwrap();
    println!("Your password is {}", pass);
}
```

You can also read a password without prompting:

```rust
extern crate rpassword;

fn main() {
    let pass = rpassword::read_password().unwrap();
    println!("Your password is {}", pass);
}
```

Finally, you can read strings with a single line, and without the terminating
newline that `read_line` would add:
```rust
extern crate rpassword;

fn main() {
    let response = rpassword::read_response().unwrap();
    println!("Your response is {}", response);
}
```

Check [examples/example.rs](examples/example.rs) for a few more examples.

## Contributors

* [@C4K3](https://github.com/C4K3)
* [@conradkleinespel](https://github.com/conradkleinespel)
* [@dcuddeback](https://github.com/dcuddeback)
* [@equalsraf](https://github.com/equalsraf)
* [@JanLikar](https://github.com/JanLikar)
* [@petevine](https://github.com/petevine)
* [@psych0d0g](https://github.com/psych0d0g)
* [@retep998](https://github.com/retep998)
* [@steveatinfincia](https://github.com/steveatinfincia)
