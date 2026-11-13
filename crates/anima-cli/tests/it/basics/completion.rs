//! `anima completion <shell>` (CMD-028).

use crate::kit::Anima;

#[tokio::test]
async fn cmd_028_completion_prints_a_script_for_each_shell() {
    for (shell, mark) in [
        ("bash", "complete"),
        ("zsh", "#compdef anima"),
        ("fish", "complete -c anima"),
    ] {
        let run = Anima::new().run(&["completion", shell]).await;
        assert_eq!(run.code, 0, "{shell}: {run:?}");
        assert!(run.stdout.contains(mark), "{shell}");
        assert!(
            run.stdout.contains("device"),
            "{shell}: the nouns are completed"
        );
    }
    let run = Anima::new().run(&["completion", "cmd.exe"]).await;
    assert_eq!(run.code, 2);
}
