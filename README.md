# Rustastic Password

[![Build Status](https://travis-ci.org/conradkleinespel/rpassword.svg?branch=master)](https://travis-ci.org/conradkleinespel/rpassword)
[![Build status](https://ci.appveyor.com/api/projects/status/h7ak407y28k0ufw2?svg=true)](https://ci.appveyor.com/project/conradkleinespel/rpassword)
[![Chat on Discord](https://img.shields.io/badge/chat-on%20discord-7289DA)](https://discord.gg/9zXtYqQ)

`rpassword` package allows you to safely read passwords in a console application on Linux, OSX and Windows.

## Supporting `rpassword`

`rpassword` is currently being maintained as a side-project, during nights and weekends, next to my full-time job. I am looking for a way to sustain my work on open source. If you find value in what I do and you would like to contribute, please consider:

- [Becoming a backer on Patreon](https://www.patreon.com/conradkleinespel)
- [Making a recurring donation on Liberapay](https://liberapay.com/conradkleinespel/)

## Usage

Add `rpassword` as a dependency in Cargo.toml:

```toml
[dependencies]
rpassword = "4.0"
```

Use `rpassword` within your code:

```rust
extern crate rpassword;

fn main() {
    // Prompt for a password on TTY (safest but not default for backwards compatibility)
    let pass = rpassword::read_password_from_tty(Some("Password: ")).unwrap();
    println!("Your password is {}", pass);
    
    // Prompt for a password on STDOUT
    let pass = rpassword::prompt_password_stdout("Password: ").unwrap();
    println!("Your password is {}", pass);

    // Prompt for a password on STDERR
    let pass = rpassword::prompt_password_stderr("Password: ").unwrap();
    println!("Your password is {}", pass);

    // Read a password without prompt
    let pass = rpassword::read_password().unwrap();
    println!("Your password is {}", pass);
}
```

The full API documentation is available at [https://docs.rs/rpassword](https://docs.rs/rpassword).

## Contributors

We welcome contribution from everyone. Feel free to open an issue or a pull request at any time.

Here's a list of existing `rpassword` contributors:

* [@C4K3](https://github.com/C4K3)
* [@conradkleinespel](https://github.com/conradkleinespel)
* [@DaveLancaster](https://github.com/DaveLancaster)
* [@dcuddeback](https://github.com/dcuddeback)
* [@Draphar](https://github.com/Draphar)
* [@equalsraf](https://github.com/equalsraf)
* [@JanLikar](https://github.com/JanLikar)
* [@joshuef](https://github.com/joshuef)
* [@longshorej](https://github.com/longshorej)
* [@petevine](https://github.com/petevine)
* [@psych0d0g](https://github.com/psych0d0g)
* [@retep998](https://github.com/retep998)
* [@steveatinfincia](https://github.com/steveatinfincia)
* [@teythoon](https://github.com/teythoon)
* [@tov](https://github.com/tov)
* [@yandexx](https://github.com/yandexx)

Thank you very much for your help!  :smiley:  :heart:

## License

The source code is released under the Apache 2.0 license.
