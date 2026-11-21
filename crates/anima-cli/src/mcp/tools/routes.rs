//! `anima_route_list` and `anima_route_get`: the public names of applications (HOST-046, task
//! H21), read only. No tool adds, checks or removes a name: making an application public on the
//! internet is a trust decision for a person with a second factor (Section 7 of the plan,
//! SF-NEW-routepublish). No tool returns a certificate or a key: the hub gives them to no API.

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::hub::Hub;
use crate::mcp::arguments::{json, object, parse};
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;
use crate::routes::documents::{RouteList, RouteShow, route_list, route_show};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteTool {
    List,
    Get,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListInput {
    /// Only the names of this application (its id); all names when absent.
    application: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetInput {
    /// The route's id (from `anima_route_list`).
    id: String,
}

const ROUTE: &str = "id, name, applicationId, deviceId, port, proof (WAITING, PROVEN or LOST), \
                     failedReads, proofRecordName and proofRecordValue (the TXT record that \
                     proves the name), machineConsent, certificateNotAfter, waitsFor (PROOF, \
                     MACHINE_CONSENT, CERTIFICATE; null when it serves), checkedAt, requests, \
                     bytesIn, bytesOut, refused, createdAt.";

impl RouteTool {
    pub const ALL: [Self; 2] = [Self::List, Self::Get];

    #[must_use]
    pub fn definition(self) -> Definition {
        match self {
            Self::List => Definition::new::<ListInput>(
                "anima_route_list",
                Access::Read,
                Effect::Reads,
                "The public names of applications that the hub serves, of one application or \
                 of each: the state of the proof of each name, what it waits for and its \
                 counts. Only a person adds or removes a name, in the client or with `anima \
                 route add`. Next: anima_route_get.",
            )
            .returns(&[
                ("routes", "array", ROUTE),
                ("count", "integer", "How many routes are listed."),
            ]),
            Self::Get => Definition::new::<GetInput>(
                "anima_route_get",
                Access::Read,
                Effect::Reads,
                "One public name of an application with its state: the TXT record of its \
                 proof, the end of its certificate and its counts. It holds no certificate \
                 and no key.",
            )
            .returns(&[("route", "object", ROUTE)]),
        }
    }

    /// Runs the tool.
    ///
    /// # Errors
    /// [`ToolError`] for wrong arguments, an unknown route or the hub's refusal.
    pub async fn call(self, hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
        match self {
            Self::List => {
                let input: ListInput = parse(arguments)?;
                let variables = route_list::Variables {
                    application_id: input.application,
                };
                let routes = hub.run::<RouteList>(variables).await?.application_routes;
                Ok(Value::Object(object([
                    ("routes", json(&routes)),
                    ("count", json!(routes.len())),
                ])))
            }
            Self::Get => {
                let input: GetInput = parse(arguments)?;
                let route = hub
                    .run::<RouteShow>(route_show::Variables { id: input.id })
                    .await?
                    .application_route
                    .ok_or_else(|| {
                        ToolError::Refused(
                            "No route has that id. Call anima_route_list.".to_owned(),
                        )
                    })?;
                Ok(Value::Object(object([("route", json(&route))])))
            }
        }
    }
}
