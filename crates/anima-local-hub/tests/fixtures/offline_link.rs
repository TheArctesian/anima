//! Native stand-in for `anima-link hub`: ready, then wait for the supervisor's stop pipe.
fn main() {
    eprintln!(
        "time=2026-11-15T10:00:00Z level=INFO msg=\"the hub link is ready\" relay=stand-in devices=0"
    );
    let mut request = String::new();
    std::io::stdin()
        .read_line(&mut request)
        .expect("read supervisor stop pipe");
}
