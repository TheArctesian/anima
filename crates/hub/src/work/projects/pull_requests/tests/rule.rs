//! The merge rule (K-E1 Section 13.3; PRJ-033, PRO-013) as a table, and its promises over every
//! combination.

use chrono::{DateTime, Utc};
use provider_github::live::pulls::{MergeableState, PullDetail, PullRequest, PullState};

use super::super::{
    merge::{Ask, Decision, Merging, Refusal, decide},
    views::MergeMethod,
};

pub(super) const HEAD: &str = "1111111111111111111111111111111111111111";
pub(super) const OTHER: &str = "2222222222222222222222222222222222222222";
const FORK: &str = "stranger/widgets";
const OWN: &str = "octo/widgets";
const ALL: [MergeMethod; 3] = [MergeMethod::Merge, MergeMethod::Squash, MergeMethod::Rebase];

pub(super) fn at() -> DateTime<Utc> {
    DateTime::from_timestamp(1_900_000_000, 0).unwrap()
}

/// A pull request as GitHub's single read gives it, from this repository.
fn pull(state: PullState, draft: bool, merge: (MergeableState, Option<bool>)) -> PullDetail {
    let text = |s: &str| s.to_owned();
    PullDetail {
        pull: PullRequest {
            number: 12,
            title: text("Title"),
            state,
            draft,
            author_login: text("octo"),
            head_branch: text("feature"),
            head_sha: text(HEAD),
            base_branch: text("main"),
            from_fork: false,
            head_repository: Some(text(OWN)),
            labels: Vec::new(),
            comments: None,
            created_at: at(),
            updated_at: at(),
            merged_at: None,
            url: text("https://github.com/octo/widgets/pull/12"),
            body: None,
        },
        mergeable: merge.1,
        mergeable_state: merge.0,
    }
}

/// One row of the table, in words: state, draft (`d` or `-`), `mergeable_state`, `mergeable`
/// (`y`, `n`, `?`), methods (`all`, `merge`, `none`), head repository (`own`, `fork`, `gone`),
/// how it merges (`one`, `allow`, `bulk`), what was typed (`-`, `fork`, `own`, `case`), the
/// actor (`user`, `agent`), `=>` and the decision.
struct Row {
    detail: PullDetail,
    methods: Vec<MergeMethod>,
    merging: (&'static str, Option<&'static str>),
    agent: bool,
    expected: Decision,
}

fn row(words: &str) -> Row {
    let w: Vec<&str> = words.split_whitespace().collect();
    let state = match w[0] {
        "open" => PullState::Open,
        "closed" => PullState::Closed,
        _ => PullState::Merged,
    };
    let mergeable = match w[3] {
        "y" => Some(true),
        "n" => Some(false),
        _ => None,
    };
    let mut detail = pull(state, w[1] == "d", (merge_state(w[2]), mergeable));
    detail.pull.from_fork = w[5] != "own";
    detail.pull.head_repository =
        (w[5] != "gone").then(|| (if w[5] == "own" { OWN } else { FORK }).to_owned());
    let methods = match w[4] {
        "all" => ALL.to_vec(),
        "merge" => vec![MergeMethod::Merge],
        _ => Vec::new(),
    };
    let typed = match w[7] {
        "fork" => Some(FORK),
        "own" => Some(OWN),
        "case" => Some("Stranger/Widgets"),
        _ => None,
    };
    let how = match w[6] {
        "allow" => "allow",
        "bulk" => "bulk",
        _ => "one",
    };
    Row {
        detail,
        methods,
        merging: (how, typed),
        agent: w[8] == "agent",
        expected: decision(w[10]),
    }
}

fn merge_state(word: &str) -> MergeableState {
    match word {
        "clean" => MergeableState::Clean,
        "unstable" => MergeableState::Unstable,
        "dirty" => MergeableState::Dirty,
        "blocked" => MergeableState::Blocked,
        "behind" => MergeableState::Behind,
        "draft" => MergeableState::Draft,
        "hooks" => MergeableState::HasHooks,
        _ => MergeableState::Unknown,
    }
}

fn decision(word: &str) -> Decision {
    let refused = match word {
        "merge" => return Decision::Merge,
        "already" => return Decision::AlreadyMerged,
        "closed" => Refusal::Closed,
        "draft" => Refusal::Draft,
        "conflict" => Refusal::Conflict,
        "blocked" => Refusal::Blocked,
        "behind" => Refusal::Behind,
        "checks" => Refusal::ChecksFailing,
        "notready" => Refusal::NotReady,
        "nomethods" => Refusal::NoMethods,
        "method" => Refusal::MethodRefused,
        "forkbulk" => Refusal::ForkInBulk,
        "unconfirmed" => Refusal::ForkUnconfirmed,
        "agent" => Refusal::ForkByAgent,
        other => panic!("no decision {other}"),
    };
    Decision::Refused(refused)
}

fn merging((how, typed): (&str, Option<&'static str>)) -> Merging<'static> {
    match how {
        "bulk" => Merging::InBulk,
        how => Merging::Alone {
            allow_failing_checks: how == "allow",
            fork_confirmed: typed,
        },
    }
}

fn ask(merging: Merging<'_>, agent: bool) -> Ask<'_> {
    Ask {
        method: MergeMethod::Squash,
        expected_head: HEAD,
        agent,
        merging,
    }
}

/// state × draft × `mergeable_state` × method allowed × fork × confirmation × agent.
const TABLE: [&str; 30] = [
    "open   - clean    y all   own  one   -    user  => merge",
    "open   - clean    y all   own  bulk  -    user  => merge",
    "open   - hooks    y all   own  one   -    user  => merge",
    "open   - clean    y all   own  one   -    agent => merge",
    "open   - clean    y all   own  bulk  -    agent => merge",
    "merged - unknown  ? all   own  one   -    user  => already",
    "merged - clean    y all   fork bulk  -    agent => already",
    "closed - clean    y all   own  one   -    user  => closed",
    "open   d draft    y all   own  one   -    user  => draft",
    "open   d clean    y all   own  one   -    user  => draft",
    "open   - dirty    n all   own  one   -    user  => conflict",
    "open   - clean    n all   own  one   -    user  => conflict",
    "open   - blocked  y all   own  allow -    user  => blocked",
    "open   - behind   y all   own  one   -    user  => behind",
    "open   - unknown  ? all   own  one   -    user  => notready",
    "open   - clean    ? all   own  one   -    user  => notready",
    "open   - unstable y all   own  one   -    user  => checks",
    "open   - unstable y all   own  allow -    user  => merge",
    "open   - unstable ? all   own  allow -    user  => notready",
    "open   - unstable y all   own  bulk  -    user  => checks",
    "open   - clean    y merge own  one   -    user  => method",
    "open   - clean    y none  own  one   -    user  => nomethods",
    "open   - clean    y all   fork one   -    user  => unconfirmed",
    "open   - clean    y all   fork one   own  user  => unconfirmed",
    "open   - clean    y all   fork one   case user  => unconfirmed",
    "open   - clean    y all   fork one   fork user  => merge",
    "open   - clean    y all   fork one   fork agent => agent",
    "open   - clean    y all   fork bulk  fork user  => forkbulk",
    "open   - clean    y all   gone one   -    user  => unconfirmed",
    "open   - unstable y all   fork one   fork user  => checks",
];

#[test]
fn prj_033_pro_013_the_merge_rule_gives_one_decision_for_each_row() {
    for words in TABLE {
        let row = row(words);
        let asked = ask(merging(row.merging), row.agent);
        assert_eq!(
            decide(&row.detail, &row.methods, &asked),
            row.expected,
            "{words}"
        );
    }
}

#[test]
fn prj_033_a_head_other_than_the_one_seen_is_head_moved_in_any_case_of_hex() {
    let detail = pull(PullState::Open, false, (MergeableState::Clean, Some(true)));
    let moved = Ask {
        expected_head: OTHER,
        ..ask(merging(("one", None)), false)
    };
    assert_eq!(
        decide(&detail, &ALL, &moved),
        Decision::Refused(Refusal::HeadMoved)
    );
    let lettered = "a".repeat(40);
    let mut read = detail;
    read.pull.head_sha.clone_from(&lettered);
    let upper = lettered.to_ascii_uppercase();
    let same = Ask {
        expected_head: &upper,
        ..ask(merging(("one", None)), false)
    };
    assert_eq!(decide(&read, &ALL, &same), Decision::Merge);
}

/// The case numbered `n` of every combination of what GitHub can say and a caller can ask.
fn case(mut n: usize) -> Row {
    let mut pick = |len: usize| {
        let i = n % len;
        n /= len;
        i
    };
    let state = ["open", "closed", "merged"][pick(3)];
    let draft = ["-", "d"][pick(2)];
    let merge = [
        "clean", "unstable", "dirty", "blocked", "behind", "draft", "hooks", "unknown",
    ][pick(8)];
    let mergeable = ["y", "n", "?"][pick(3)];
    let methods = ["all", "merge", "none"][pick(3)];
    let head = ["own", "fork", "gone"][pick(3)];
    let how = ["one", "allow", "bulk"][pick(3)];
    let typed = ["-", "fork", "own"][pick(3)];
    let actor = ["user", "agent"][pick(2)];
    let words = format!(
        "{state} {draft} {merge} {mergeable} {methods} {head} {how} {typed} {actor} => merge"
    );
    row(&words)
}

#[test]
fn pro_013_prj_033_the_rule_merges_only_what_it_promises_over_every_combination() {
    for n in 0..(3 * 2 * 8 * 3 * 3 * 3 * 3 * 3 * 2) {
        let row = case(n);
        let asked = ask(merging(row.merging), row.agent);
        let decision = decide(&row.detail, &row.methods, &asked);
        let (read, how) = (&row.detail.pull, row.merging.0);
        if read.state == PullState::Merged {
            assert_eq!(decision, Decision::AlreadyMerged, "case {n}");
            continue;
        }
        if decision != Decision::Merge {
            continue;
        }
        assert!(read.state == PullState::Open && !read.draft, "case {n}");
        assert_eq!(row.detail.mergeable, Some(true), "case {n}");
        assert!(row.methods.contains(&MergeMethod::Squash), "case {n}");
        if read.from_fork {
            assert!(
                how != "bulk" && !row.agent,
                "a fork in bulk or by an agent: case {n}"
            );
            assert_eq!(read.head_repository.as_deref(), row.merging.1, "case {n}");
        }
        match row.detail.mergeable_state {
            MergeableState::Clean | MergeableState::HasHooks => {}
            MergeableState::Unstable => assert_eq!(how, "allow", "case {n}"),
            _ => panic!("merged one that cannot merge: case {n}"),
        }
    }
}
