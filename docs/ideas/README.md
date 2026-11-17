# Ideas

| Item | Value |
| --- | --- |
| Document | How to record an idea before it is a proposal (PLN-018) |
| Version | 1.0 |
| Date | 2026-11-17 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | Approved (operator, 2026-11-17) |

This folder is the first stage of the path in [plan/rules.md](../plan/rules.md), PLN-018. The folders in this folder are the list of the ideas. There is no second list.

## Rules

- Each idea is one folder: `docs/ideas/<short-name>/`. The name is lower case with hyphens.
- The folder has one file `idea.md`. It gives: the idea in one paragraph, who had it and the date, the source (a chat, the Grok bot, a note of the business vault, a different product), and the questions that are open.
- The folder can hold sketches, mockups and notes of the idea. A mockup of an idea is not in `docs/mockup/` (PLN-019).
- An idea has no requirement identifier, no row in the register and no tier. Do not build from an idea (PLN-001).
- A review of an idea adds to `idea.md`. It gives the date and what changed.
- An idea stops when it becomes a proposal or when the operator drops it. The commit that writes the proposal, or that records the drop, deletes the folder. The history of the repository keeps the text.
- Do not put a secret, a price that is not public or a finding of the business vault here. This repository is public.

## How an idea gets here

An idea comes from one of three places. In each case, a session writes the folder in a commit that changes only `docs/ideas/`. The commit needs only `bun run quality:documents`.

| Source | What the session does |
| --- | --- |
| The operator tells it to a coding session and does not ask for a build | The session writes the folder and stops. It does not build. If the operator asks for a build, the session writes the proposal first (PLN-013); the idea folder is then not necessary. |
| The Grok bot, or a different chat | The operator gives the text of the bot to a session. The session writes the folder and puts the text below "Source text", with no change. |
| A note of the business vault | The private part stays in the vault. The folder here holds only the part that is about the product, and gives the title of the vault note as its source. |

## The file `idea.md`

```text
# <The idea in a few words>

- From: <the operator, the Grok bot, a note of the vault>, <date>
- Serves: <the row of work plan Section 4 that it can serve, or "not known">
- Owner if it becomes work: <the plan that can own it (work plan Section 5), or "a new proposal">

## The idea

<One paragraph.>

## Open questions

- <A question that the operator must answer before this can be a proposal.>

## Reviews

- <date>: <what changed in this round, and what the operator said.>

## Source text

<The text of the chat or of the bot, with no change. Remove each secret.>
```

## Review rounds, and when an idea is ready

- Each round is one entry below "Reviews": a session or the operator reads the idea, answers or adds questions, and writes what changed. More than one round is good: each round makes the idea more exact.
- If a plan already owns the subject, the idea becomes a change to that plan, not a new proposal (PLN-004).
- An idea is ready to be a proposal when its "Serves" line names a row, and each open question has an answer or is written as an open decision for the proposal.
- A session that starts and finds no task for an agent can do one review round of an idea. It does not make the idea a proposal without the operator.

## Ideas now

None. The first idea makes the first folder.
