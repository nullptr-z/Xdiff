use atty::*;
fn main() {
    // test atty crate
    if atty::is(atty::Stream::Stdout) {
        println!("stdout is a tty");
    } else {
        println!("stdout is not a tty");
    }
}
