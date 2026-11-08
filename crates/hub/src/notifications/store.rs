//! All SQL on schema `notifications`: I/O only; rows are mapped in `rows`. Ids and time come
//! in as parameters.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        unused_imports,
        reason = "the channel, decision and dispatcher tasks of WP-45 (T3 to T6) call the rest"
    )
)]

mod channels;
mod check_choices;
mod deliveries;
mod outbox;
mod rows;
mod settings;

pub(super) use channels::{
    delete_channel, insert_channel, list_channels, on_by_default, update_channel,
};
pub(super) use check_choices::{
    check_channels, check_settings, lock_check_state, set_check_channels, write_check_settings,
    write_check_state,
};
pub(super) use deliveries::{
    create_days_ahead, database_now, deliveries_page, drop_days_before, insert_delivery,
    last_deliveries,
};
pub(super) use outbox::{
    claim_due, delete_finished_before, give_up, insert_outbox, mark_sent, reschedule,
};
pub(super) use settings::{installation_settings, write_installation_settings};
