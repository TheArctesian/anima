//! The two routes of the work route that model work uses (K10 Section 2; model work
//! Section 3): the claim and the report. The daemon asks; the hub never opens a connection.

use daemon_core::wire::{Claim, DaemonToken, Reported};
use serde_json::Value;

use super::answer::Answer;
use super::{Call, HubClient, json, read_json};

impl HubClient {
    /// `POST /work/v1/claim`: `None` for `204`, the item of a `200` as JSON. The item is read
    /// and checked by the caller, which knows the fixed lists.
    pub async fn claim_work(&self, token: &DaemonToken, claim: &Claim) -> Answer<Option<Value>> {
        let call = Call {
            route: "work/v1/claim",
            bearer: Some(token),
            body: json(claim),
            ok: &[204, 200],
        };
        self.post(call, |status, body| {
            if status == 204 {
                Some(None)
            } else {
                read_json::<Value>(body).map(Some)
            }
        })
        .await
    }

    /// `POST /work/v1/report` with a report of a model item.
    pub async fn report_work(&self, token: &DaemonToken, report: &Value) -> Answer<Reported> {
        let call = Call {
            route: "work/v1/report",
            bearer: Some(token),
            body: json(report),
            ok: &[200],
        };
        self.post(call, |_, body| read_json(body)).await
    }
}
