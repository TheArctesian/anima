//! The drives of a Linux device over SSH (MET-027, MET-028): stored on the first round and when
//! they change, kept when `lsblk` fails, read with the older column on old util-linux, and
//! the activity of each drive from the second round.

mod rounds;
