//! The desktop program. The shell is the library, so that a phone can load it (MOB-001).
//! On a phone the operating system starts the library, and this program does nothing.

fn main() {
    #[cfg(desktop)]
    anima_app_lib::run();
}
