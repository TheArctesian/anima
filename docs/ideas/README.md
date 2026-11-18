# Ideas

| Item | Value |
| --- | --- |
| Document | How to record an idea before it is a proposal (PLN-018) |
| Version | 1.1 |
| Date | 2026-11-18 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | Approved (operator, 2026-11-17). Version 1.1 adds the sections "Who writes ideas" to "How the Planner makes an idea a proposal", and waits for the approval of the operator. |

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
- Source: <the title of the note of the vault, or the chat. Give the title only. Do not link to the vault.>
- Status: <Raw, Framed or Ready for Planner>
- Serves: <the row of work plan Section 4 that it can serve, or "not known">. Add "not confirmed" if no one has checked the row.
- Owner if it becomes work: <the plan that can own it (work plan Section 5), or "a new proposal">
- Effort: <S, M or L, an estimate>. Recommendation: <should, could or skip>

## The idea

<One paragraph.>

## Evidence

- <A source with its date, such as a public page, a document of this repository, or a chat. Mark a source or a date that is not verified.>

## Overlap check

- <What the documents ownership.md, register.md and deferred.md say about this subject. Give the identifiers. Give the date of the check.>

## Narrowest version, and when to stop

- Narrowest version: <the smallest work that shows the idea is good.>
- Stop if: <the result that means the operator drops the idea.>

## Open questions

- <A question that the operator must answer before this can be a proposal. Write it so that the answer is yes, no or one choice.>

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

## Who writes ideas

The bot Anima Product Ideas writes the ideas of this folder. It takes research from the other bots and raw ideas from the operator. The operator or a coding session can also write an idea (see "How an idea gets here").

The bot writes only the idea stage. It does not write a proposal, a plan, a specification or a task breakdown. The Planner writes those. The other bots read the ideas from this folder: this folder is the one list, and the vault does not hold a second list.

## Private working notes

The business vault is a private repository. Its folder `Product/Ideas/` holds the private working notes of each idea. A note there can hold prices, costs, funding, trademark findings, legal doubts and notes about people. None of these items enters this repository (see "Public-safe rules").

The private note and the folder here have one subject. The private note gives the path of the folder here. The folder here gives the title of the private note as its source in the line "Source".

## Links go one way

The vault can link to this repository. This repository must not link to the vault.

- A vault note links to `idea.md` here. It uses the form `[[Product/anima/docs/ideas/<name>/idea|<Title>]]`, because the vault holds this repository as a read-only copy in `Product/anima`.
- A file here names the vault note by its title, as plain text. It does not use a link, a path or an address of the vault.
- The vault copy of this repository is pinned to one commit. It shows a new idea after the vault moves to a newer commit.

## Public-safe rules

This repository is public. Each file in this folder must follow these rules:

- Do not write a secret, a price, a cost, a funding finding, a trademark finding or a private finding of the vault.
- Do not write personal data: a name of a private person, an address, a number or an account.
- Name a competitor only with facts that are public. Give the public source and its date.
- Keep the private part of an idea in the vault. Write in the idea only the part that is about the product.
- Write the words of the operator with no change, and remove each secret from them. Mark the words that another agent relayed as relayed.
- If you are not sure that a fact is public, keep it in the vault.

## Statuses

An idea has one status in its line "Status". Each status says what comes next.

| Status | Meaning | Next |
| --- | --- | --- |
| Raw | The words are logged as the operator said them. No one has framed the idea. | Frame it, with at most three questions. |
| Framed | The idea has the sections of the template, as far as the sources allow. It does not meet the ready bar. | Close the gaps of the ready bar, or ask the operator. |
| Ready for Planner | The idea meets the ready bar. | The bot tells the Planner the path of the folder. |

Two more states end an idea. Both delete the folder (PLN-018), so no file here holds them.

- Handed off: the Planner makes the idea a proposal, or a change to a plan. The commit deletes the folder.
- Dropped: the operator drops the idea. The commit that records the drop deletes the folder.

The vault note keeps the state Handed off, with the path of the proposal, or the state Dropped, with the reason.

## The ready bar

An idea is Ready for Planner when all four points are true:

1. The line "Serves" names a row of Section 4 of [plan/priorities.md](../plan/priorities.md), and someone has checked the row. A row that no one has checked is "not confirmed", and the idea is not ready.
2. The section "Overlap check" is done against [plan/ownership.md](../plan/ownership.md), [plan/register.md](../plan/register.md) and [deferred.md](../deferred.md). It names each requirement identifier or ruling that overlaps.
3. The section "Evidence" has at least one source with a date.
4. There are at most three open questions. Each question has the answer yes or no, or one choice from a short list.

This bar is for the hand-off. The rule of "Review rounds, and when an idea is ready" stays: when a session writes the proposal, each open question has an answer or is written as an open decision.

## How the Planner makes an idea a proposal

The Planner starts from an idea that is Ready for Planner. It does these steps:

1. Check the overlap. If a plan in the register owns the subject, change that plan and do not write a new proposal (PLN-004).
2. If no plan owns it, write one proposal in `docs/proposals/` with its header and its row in Section 9 (PLN-002, PLN-020). Name the row of Section 4 that it serves (PLN-003).
3. Write the requirements with identifiers. Write each open question of the idea as a row of the table of open decisions, with one question and a recommendation (PLN-018).
4. Delete the folder of the idea in the same commit. The history of the repository keeps the text.
5. Run `bun run quality:register` and `bun run quality:documents`.
6. Tell the bot the path of the proposal. The bot records the state Handed off in the vault note.

The proposal is a draft. A session builds only the parts that a ruling names (PLN-007).

## Ideas now

The folders in this directory are the list. This document does not copy the list (PLN-016).
