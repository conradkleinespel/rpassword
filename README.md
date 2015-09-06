# Rustastic Password

This [Rust](http://www.rust-lang.org/) package allows you to safely read
passwords from standard input in a console application.

## Usage

Add termios as a dependency in Cargo.toml:

```toml
[dependencies]
rpassword = "0.0.5"
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
