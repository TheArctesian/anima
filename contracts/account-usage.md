# The figures of an account for the page of a hosted hub

Status: **proposed**, version 0.4, 2026-11-20. HMD-073 in form B of [hosted-mode.md](../docs/proposals/hosted-mode.md) Section 6.11 (approved, R-NEW-hubusage) says that the page of the hub takes the figures of the account from the account, "in one form, which the contract defines once". This file is that form, as the sessions of the pool (`anima-cloud`), of the account site (`anima-site`) and of the hub agreed it on 2026-11-20. The body is the answer of the pool for an account and a month; the pool owns its members. It stays proposed until the route and the path are built; it changes only by a ruling after that (CON-001). Task HM-T38 builds the client against it.

## 1. Who answers, and to whom

| Way in | Request | Who answers | Sign-in |
| --- | --- | --- | --- |
| A browser at the address of the hub | `GET /_door/usage` on the origin of the hub, no parameter | The door of the pool, itself. It never passes this path to the hub. | The session of the door (its cookie) |
| The desktop and phone application | `POST <cloud>/v1/hub/usage`, no body, beside `/v1/hub` and `/v1/hub/access` | The account site, which gives the body of the pool with no change and adds `owes` | The bearer token of the account that the shell holds |

The request names no account, no hub and no month: each way in answers only for the account of its sign-in and for the month of now. The hub takes no part: it holds no figure of the plan, of the hours or of an amount, and it serves no route of this contract.

## 2. The answer

`200` with a JSON body; `Cache-Control: no-store` (the door keeps it at most 30 seconds):

```json
{
  "period": "2026-11",
  "until": 1795161600,
  "currency": "usd",
  "lines": [
    { "size": "solo", "plan": "hobby", "kind": "running", "seconds": 45000, "hours": "12.5", "amount": 412 },
    { "size": "solo", "plan": "hobby", "kind": "asleep", "seconds": 1800, "hours": "0.5", "amount": 0 }
  ],
  "total": 412,
  "prices_set": true,
  "hubs": [
    { "id": "h1", "state": "running", "size": "solo", "plan": "hobby", "disk_limit_bytes": 10000000000, "disk_used_bytes": 2500000000 }
  ],
  "owes": false
}
```

| Member | Meaning | How the page uses it |
| --- | --- | --- |
| `period` | The month of the figures, `YYYY-MM`. | Read; not shown. |
| `until` | The end of the counted time, in Unix seconds, at most now. | The time of the figures. |
| `currency` | The currency of `amount` and `total`, three letters (ISO 4217, any case). | Made upper case. |
| `lines[].kind`, `lines[].seconds` | Whole seconds of the month up to `until`, by kind `running` or `asleep`. | Hours awake: the sum of the `seconds` of kind `running`. Hours asleep: of kind `asleep`. |
| `total` | The amount of the month so far, before tax, in the smallest unit of `currency`. It comes from the usage record of the pool, from which the invoice comes. | The amount, when the plan is charged and `prices_set` is `true`. |
| `prices_set` | `false` when the pool has no prices. | With `false` and a charged plan, the amount is "Not available just now" and the estimate note is not shown. |
| `hubs[]` | The hubs of the account, as they are now, whatever the period. | The hub of the page: the entry whose `id` is the first label of the host of the page (`https://<hub id>.hub.anima.ing`), else the only entry. With several entries and none named, the figures are "Not available just now". |
| `hubs[].plan` | The plan of the hub. `free` is not charged. | The name of the plan, with a capital first letter. With `free`, "Nothing is charged on this plan" in the place of an amount. |
| `hubs[].disk_limit_bytes` | The disk limit of the plan. | "Disk used of the plan". |
| `hubs[].disk_used_bytes` | The bytes that the quota counts as used, the system of the hub included; `null` when the pool cannot read them. | With `null`, the page shows the limit and no percentage. This figure is not the "data of your hub" that the hub measures (HMD-072); the page shows the two apart. |
| `owes` | Added by the site: `true` when the account owes an amount of an earlier month. No sum. | With `true`, the page points to the account page. A missing `owes` is `false`. |

A member that the client does not use (`account`, `lines[].size`, `lines[].hours`, `lines[].amount`, `hubs[].state`, `hubs[].size` and any new member) is ignored. A body without a member that the page uses, or with such a member of another kind, is shown as "Not available just now".

## 3. The failures

| Way in | Status | The page shows |
| --- | --- | --- |
| Both | `401` (`sign_in_needed` at the door, `unauthorized` at the site): no session of the door, or no token or an expired one in the application | That the person signs in to the account to see the figures |
| The door | `503` `usage_unavailable`: the pool gives no figures | "Not available just now" |
| The site | `403` `sign_in_method`: a sign-in method that may not open its account | "Not available just now", until the words of Connect GitHub are built |
| The site | `409` `not_ready` (no hub asked for yet, or the pool does not know the account), `502` `cloud_unavailable`, `503` `not_configured` | "Not available just now" |
| Both | Any other status, or no answer in 10 seconds | "Not available just now" |

An error body has the form `{"error": {"code": ...}}`; the client reads only the status. No answer holds a figure of another account or of the server, and no log line holds a figure of this answer (LOG-007).
