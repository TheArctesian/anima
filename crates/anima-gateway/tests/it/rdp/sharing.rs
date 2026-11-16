//! The clipboard of an RDP session under the device's switches (RA-033), and when the
//! device is asked for a screen of the size of the client's view (RA-004).

use std::time::Duration;

use anima_gateway::rdp::{Reply, Said, Sharing, ViewRequests, reply, screen_for};

const BOTH: Sharing = Sharing {
    copy: true,
    paste: true,
};
const COPY_ONLY: Sharing = Sharing {
    copy: true,
    paste: false,
};
const PASTE_ONLY: Sharing = Sharing {
    copy: false,
    paste: true,
};
const NEITHER: Sharing = Sharing {
    copy: false,
    paste: false,
};

fn copied() -> Said {
    Said::Text("from the device".to_owned())
}

#[test]
fn with_copy_off_the_devices_text_is_not_asked_for_and_not_passed_on() {
    for sharing in [PASTE_ONLY, NEITHER] {
        assert_eq!(reply(Said::CopiedText, sharing, None), Reply::Nothing);
        // Text the device sends though nobody asked.
        assert_eq!(reply(copied(), sharing, None), Reply::Nothing);
    }
    for sharing in [BOTH, COPY_ONLY] {
        assert_eq!(reply(Said::CopiedText, sharing, None), Reply::AskForText);
        assert_eq!(
            reply(copied(), sharing, None),
            Reply::ToClient("from the device".to_owned())
        );
    }
}

#[test]
fn with_paste_off_the_device_learns_of_no_text_and_gets_none() {
    let text = Some("from the client");
    for sharing in [COPY_ONLY, NEITHER] {
        assert_eq!(reply(Said::WhatIsThere, sharing, text), Reply::Offer(false));
        assert_eq!(reply(Said::WantsText, sharing, text), Reply::GiveText(None));
    }
    for sharing in [BOTH, PASTE_ONLY] {
        assert_eq!(reply(Said::WhatIsThere, sharing, text), Reply::Offer(true));
        assert_eq!(reply(Said::WantsText, sharing, text), Reply::GiveText(text));
    }
}

#[test]
fn a_client_with_no_text_offers_none_and_answers_a_paste_with_none() {
    assert_eq!(reply(Said::WhatIsThere, BOTH, None), Reply::Offer(false));
    assert_eq!(reply(Said::WantsText, BOTH, None), Reply::GiveText(None));
}

const fn at(milliseconds: u64) -> Duration {
    Duration::from_millis(milliseconds)
}

#[test]
fn the_device_is_asked_once_the_view_has_kept_its_size() {
    let mut requests = ViewRequests::default();
    assert_eq!(requests.ask_at(), None);

    requests.view_changed((1000, 700), at(0));
    requests.view_changed((1200, 800), at(200));

    // The first size never comes due; the second does 300 ms after it.
    assert_eq!(requests.ask_at(), Some(at(500)));
    assert_eq!(requests.due((640, 480), at(499)), None);
    assert_eq!(requests.due((640, 480), at(500)), Some((1200, 800)));
    requests.asked();
    assert_eq!(requests.ask_at(), None);
    assert_eq!(requests.due((640, 480), at(900)), None);
}

#[test]
fn a_view_of_the_screens_size_asks_nothing_and_an_odd_width_counts_as_the_even_one() {
    assert_eq!(screen_for((1001, 700)), (1000, 700));
    assert_eq!(screen_for((50, 9000)), (200, 8192));
    let mut requests = ViewRequests::default();

    requests.view_changed((1001, 700), at(0));

    assert_eq!(requests.due((1000, 700), at(300)), None);
    assert_eq!(requests.ask_at(), None);
}

#[test]
fn a_device_that_cannot_be_asked_is_asked_again_five_times_and_then_left() {
    let mut requests = ViewRequests::default();
    requests.view_changed((1200, 800), at(0));
    let mut now = at(300);
    for _ in 0..5 {
        assert_eq!(requests.due((640, 480), now), Some((1200, 800)));
        requests.not_asked(now);
        now += Duration::from_secs(1);
        assert_eq!(requests.ask_at(), Some(now));
    }
    assert_eq!(requests.due((640, 480), now), Some((1200, 800)));
    requests.not_asked(now);

    assert_eq!(requests.ask_at(), None);
}
