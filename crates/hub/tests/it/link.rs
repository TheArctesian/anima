//! The device link through the hub and a fake control socket of `anima-link hub`:
//! `linkDevices`, `createLinkInvite` and `revokeLinkDevice`, their token rules, their audit
//! entries and their errors (FRN-009, LNK-003, LNK-004, LOG-005, LOG-007, D-23).

mod control;
mod devices;
mod hosted;
mod hub_device;
mod invites;
mod presence;
mod refusals;
mod unaudited;
