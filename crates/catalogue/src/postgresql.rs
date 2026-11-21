//! The knowledge of the PostgreSQL entry as pure functions (R-216, Section 14 of the proposal,
//! task H24): the names of the database and of the user of an application, the fixed
//! statements that make and delete them, the connection values in the variables that a
//! recipe names, and the commands of the copy, the way back and the test of the way back.
//!
//! Nothing here runs a command, reaches a machine or makes a password. The applications
//! program on the machine makes the password and runs the statements and the commands
//! (OD-HOST-42 A): here the password is only an input value. No statement holds text of the
//! hub: each identifier is a checked name, quoted, and the password is a quoted literal of
//! printable characters with no quote. There is no replica (OD-HOST-38 is open).

mod connection;
mod copies;
mod names;
mod statements;
#[cfg(test)]
mod tests;

pub use connection::{Address, Value, values};
pub use copies::{
    DumpFile, Restore, check_command, dump_command, list_command, missing_tables, restore_command,
    tables_in_list, tables_query, way_back, way_back_test,
};
pub use names::{DatabaseName, NameError, Names, is_extension};
pub use statements::{
    Password, PasswordError, Setup, Statement, drop_database, new_database, removal, setup,
    way_back_database,
};
