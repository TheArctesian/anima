//! The native Windows gate runner; deliberately absent from production packaging.
fn main() -> std::process::ExitCode {
    #[cfg(windows)]
    match anima_database::test_server::run(&std::env::args_os().skip(1).collect::<Vec<_>>()) {
        Ok(status) if status.success() => std::process::ExitCode::SUCCESS,
        Ok(_) => std::process::ExitCode::FAILURE,
        Err(_) => {
            eprintln!("test database: the private runtime or gate could not complete");
            std::process::ExitCode::FAILURE
        }
    }
    #[cfg(not(windows))]
    {
        eprintln!("Use quality/bin/with-postgres.ts on this platform.");
        std::process::ExitCode::FAILURE
    }
}
