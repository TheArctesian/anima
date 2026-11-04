#[test]
fn print_dummy() {
    let pw: [u8; 32] = super::login::random().ok().unwrap();
    let salt = super::login::random::<16>().ok().unwrap();
    let text: String = pw.iter().map(|b| format!("{b:02x}")).collect();
    println!("DUMMY={}", super::password::hash(&super::Password::new(text), &salt).unwrap());
}
