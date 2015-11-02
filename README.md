# Rustastic Password

[![Build status](https://travis-ci.org/conradkleinespel/rustastic-password.svg?branch=master)](https://travis-ci.org/conradkleinespel/rustastic-password)
[![Build status](https://ci.appveyor.com/api/projects/status/812odw3tw6oec5sw/branch/master?svg=true)](https://ci.appveyor.com/project/conradkleinespel/rustastic-password/branch/master)

This [Rust](http://www.rust-lang.org/) package allows you to safely read
passwords from standard input in a console application.

You can build the documentation with `cargo doc` or [view it online](http://conradk.com/rustastic-password/target/doc/rpassword/).

## Usage

Add `rpassword` as a dependency in Cargo.toml:

```toml
[dependencies]
rpassword = "0.1"
```

Import the `rpassword` crate and use the `read_password()` function:

```rust
extern crate rpassword;

use rpassword::read_password;

fn main() {
    print!("Type a password: ");
    let password = read_password().unwrap();
    println!("The password is: '{}'", password);
}
```

## Contributors

* [@conradkleinespel](https://github.com/conradkleinespel)
* [@dcuddeback](https://github.com/dcuddeback)
* [@equalsraf](https://github.com/equalsraf)
* [@retep998](https://github.com/retep998)
