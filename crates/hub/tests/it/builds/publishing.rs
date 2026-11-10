//! Publishing end to end against an in-process R2 (BLD-060 to BLD-070, D-24 to D-28, R-108):
//! the builder uploads through the hub's presigned URLs, and the publisher checks the files
//! and writes the stable name and `latest.json`.

mod fake_r2;
mod fences;
mod flow;
mod grants;
mod origin;
mod refusals;
mod setup;
mod signing;
