// Copyright 2014-2017 The Rpassword Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate rpassword;

fn main() {
    // Password reading functions
    match rpassword::read_password() {
        Ok(pass) => println!("{}", pass),
        Err(_) => println!("error")
    }
    match rpassword::prompt_password_stdout("prompt_password_stdout") {
        Ok(pass) => println!("{}", pass),
        Err(_) => println!("error")
    }
    match rpassword::prompt_password_stderr("prompt_password_stderr") {
        Ok(pass) => println!("{}", pass),
        Err(_) => println!("error")
    }

    // Regular input reading functions, deprecated but present for BC
    match rpassword::read_response() {
        Ok(pass) => println!("{}", pass),
        Err(_) => println!("error")
    }
    match rpassword::prompt_response_stdout("prompt_reply_stdout") {
        Ok(pass) => println!("{}", pass),
        Err(_) => println!("error")
    }
    match rpassword::prompt_response_stderr("prompt_reply_stderr") {
        Ok(pass) => println!("{}", pass),
        Err(_) => println!("error")
    }
}
