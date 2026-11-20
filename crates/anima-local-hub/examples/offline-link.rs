//! Offline integration-test fixture, equivalent to the Unix tests' shell stand-in.
//! Never included in an application bundle; the real link has its own relay tests.
fn main() {
    eprintln!(
        "time=2026-11-20T00:00:00Z level=INFO msg=\"the hub link is ready\" relay=stand-in devices=0"
    );
    let _ = std::io::copy(&mut std::io::stdin().lock(), &mut std::io::sink());
}
