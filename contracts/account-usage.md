# The figures of an account for the page of a hosted hub

Status: **proposed**, version 0.1, 2026-11-20. HMD-073 in form B of [hosted-mode.md](../docs/proposals/hosted-mode.md) Section 6.11 (approved, R-NEW-hubusage) says that the page of the hub takes the figures of the account from the account, "in one form, which the contract defines once". This file is that form. The session of the hub and the client sent it to the sessions of the pool (`anima-cloud`) and of the account site (`anima-site`) on 2026-11-20; it is proposed until both agree, and it changes only by a ruling after that (CON-001). Task HM-T38 builds the client against it.

## 1. Who answers, and to whom

| Way in | Request | Who answers | Sign-in |
| --- | --- | --- | --- |
| A browser at the address of the hub | `GET /account/v1/usage` on the origin of the hub | The gateway of the pool, itself. It never passes this path to the hub. | The session of the gateway (its cookie) |
| The desktop and phone application | `GET <cloud>/v1/hub/usage`, beside `/v1/hub` and `/v1/hub/access` | The account site, which gives the answer of the pool with no change | The bearer token of the account that the shell holds |

The request names no account, no hub and no month: each way in answers only for the account of its sign-in and for the month of now. The hub takes no part: it holds no figure of the plan, of the hours or of an amount, and it serves no route of this contract.

## 2. The answer

`200` with a JSON body, `Content-Type: application/json`, `Cache-Control: private, max-age=60`:

```json
{
  "at": "2026-11-20T18:00:00Z",
  "month": "2026-11",
  "plan": { "name": "Hobby", "charged": true },
  "awakeSeconds": 45000,
  "asleepSeconds": 1800,
  "amount": { "minor": 412, "currency": "USD" },
  "owes": false,
  "disk": { "limitBytes": 10000000000, "usedBytes": 2500000000 }
}
```

| Member | Meaning |
| --- | --- |
| `at` | When the pool computed the figures (RFC 3339). The page shows it. |
| `month` | The month of the figures, `YYYY-MM`. |
| `plan.name` | The name of the plan of the account, as the account page writes it. Not empty. |
| `plan.charged` | `false` on a plan that is not charged: the page shows "Nothing is charged on this plan" in the place of an amount. |
| `awakeSeconds`, `asleepSeconds` | The time that the hub of the account was awake and asleep in the month so far. Whole numbers, 0 or more. |
| `amount` | The amount of the month so far, before tax, in the smallest unit of `currency` (cents for `USD`), and `currency` as three capital letters (ISO 4217). `null` when `plan.charged` is `false`. It comes from the usage record of the pool, from which the invoice comes. |
| `owes` | `true` when the account owes an amount of an earlier month. The page points to the account page and shows no sum of it. |
| `disk.limitBytes` | The disk limit of the plan. |
| `disk.usedBytes` | The bytes that the quota of the plan counts as used, the system of the hub included. `null` when the pool cannot read them; the page then shows the limit and no percentage. This figure is not the "data of your hub" that the hub measures (HMD-072); the page shows the two apart. |

A member that the client does not know is ignored. A body without one of these members, or with a member of another kind, is shown as "Not available just now".

## 3. The failures

| Status | Meaning | The page shows |
| --- | --- | --- |
| `401` | No sign-in of the account: no session of the gateway, or no token or an expired one in the application | That the person signs in to the account to see the figures |
| `404` | The account has no hosted hub | "Not available just now" |
| Any other, or no answer in 10 seconds | The pool or the site cannot answer now | "Not available just now" |

An error body, when there is one, has the form of the site's other routes (`{"error": {"code": ...}}`); the client reads only the status. No answer holds a figure of another account or of the server, and no log line holds a figure of this answer (LOG-007).
