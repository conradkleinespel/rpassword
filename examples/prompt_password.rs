use rpassword::prompt_password;

fn main() {
    let password = prompt_password("Password: ").unwrap();
    println!("The password is: {}", password);
}
