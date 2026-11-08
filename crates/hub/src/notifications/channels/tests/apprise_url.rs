//! N-4: which stateless Apprise URLs the hub hands to Apprise, both ways. Every URL here is
//! synthetic.

use crate::notifications::channels::{apprise_url::Refusal, permitted_apprise_url as permitted};

#[test]
fn n_4_urls_of_fixed_host_services_with_display_keys_are_permitted() {
    for url in [
        "discord://1234/webhook-token",
        "DISCORD://1234/webhook-token",
        "tgram://123:bot-token/-100200300?format=markdown&silent=yes",
        "slack://TA/TB/TC/#general?footer=no;blocks=yes",
        "pover://user@token?priority=1&sound=bike&retry=60&expire=600",
        "pbul://access-token",
        "gchat://workspace/key/token?thread=ops",
        "pagerduty://integration@api-key",
        "opsgenie://api-key/@team?FORMAT=text&Priority=3",
        "discord://1234/token?%66ormat=text",
        "discord://1234/token?",
        "discord://1234/token?tts=no&&avatar=no",
    ] {
        assert_eq!(permitted(url), Ok(()), "{url}");
    }
}

#[test]
fn n_4_schemes_that_post_to_any_host_are_refused() {
    for url in [
        "json://evil.example.test/hook",
        "jsons://evil.example.test/hook",
        "form://evil.example.test/",
        "forms://evil.example.test/",
        "xml://evil.example.test/",
        "xmls://evil.example.test/",
        "apprise://evil.example.test/key",
        "apprises://evil.example.test/key",
        "http://evil.example.test/",
        "https://discord.com/api/webhooks/1/t",
        "ntfy://ntfy.example.test/topic",
        "gotify://gotify.example.test/token",
        "mailto://user:pass@mail.example.test",
        "matrix://user:pass@matrix.example.test",
        "mmost://mattermost.example.test/token",
        "rocket://user:pass@rocket.example.test",
        "mqtt://broker.example.test/topic",
        "syslog://syslog.example.test",
        "file:///etc/passwd",
        "windows://",
        "dbus://",
        "discordx://1234/token",
    ] {
        assert_eq!(permitted(url), Err(Refusal::Scheme), "{url}");
    }
}

#[test]
fn n_4_query_keys_that_read_files_turn_off_tls_or_redirect_are_refused() {
    for url in [
        "discord://1234/token?template=/etc/passwd",
        "discord://1234/token?attach=/etc/passwd",
        "discord://1234/token?attachment=evil.example.test",
        "discord://1234/token?verify=no",
        "discord://1234/token?VERIFY=no",
        "discord://1234/token?%76erify=no",
        "discord://1234/token?avatar_url=evil.example.test/a.png",
        "discord://1234/token?image=yes",
        "discord://1234/token?url=evil.example.test",
        "discord://1234/token?href=evil.example.test",
        "discord://1234/token?cto=1&rto=1",
        "discord://1234/token?store=yes",
        "discord://1234/token?+X-Header=1",
        "discord://1234/token?-name=1",
        "discord://1234/token?:key=1",
        "discord://1234/token?format=text;verify=no",
        "discord://1234/token?format%",
        "discord://1234/token?%ff=1",
        "discord://1234/token?%20format=text",
        "discord://1234/token?formatx=text",
        "discord://1234/token?=text",
    ] {
        assert_eq!(permitted(url), Err(Refusal::Key), "{url}");
    }
}

#[test]
fn n_4_urls_apprise_would_split_or_read_twice_are_refused() {
    let long = format!("discord://1234/{}", "t".repeat(2040));
    for url in [
        "",
        "discord://1234/token json://evil.example.test/",
        "discord://1234/token,json://evil.example.test/",
        "discord://1234/json://evil.example.test/",
        "discord://1234/token?attach=file:///etc/passwd",
        "discord://1234/token?format=text?verify=no",
        "discord://1234/token\n",
        "discord://1234/token\tx",
        "disc ord://1234/token",
        "discord://1234/to\u{200b}ken",
        "discord:1234/token",
        long.as_str(),
    ] {
        assert_eq!(permitted(url), Err(Refusal::Shape), "{url:?}");
    }
    assert_eq!(
        permitted(&format!("discord://1234/{}", "t".repeat(2033))),
        Ok(())
    );
}

/// Defence in depth: each URL below is permitted without its comma, so the comma rule alone
/// refuses it.
#[test]
fn n_4_a_comma_is_refused_when_it_is_the_only_fault() {
    for (with_comma, without) in [
        ("discord://1234/to,ken", "discord://1234/token"),
        ("discord://1234/token,", "discord://1234/token"),
        ("tgram://1:a/2?to=a,b", "tgram://1:a/2?to=ab"),
    ] {
        assert_eq!(permitted(without), Ok(()), "{without}");
        assert_eq!(permitted(with_comma), Err(Refusal::Shape), "{with_comma}");
    }
}

#[test]
fn log_007_a_refusal_never_quotes_the_url() {
    for refusal in [Refusal::Shape, Refusal::Scheme, Refusal::Key] {
        assert!(!refusal.message().contains("://"));
    }
}
