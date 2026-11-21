//! The repair hint for a tool or a backend that is not usable: a fixed sentence chosen by the
//! tool or backend and the reason, never text from the device (CAP-002, CAP-024).

use super::documents::capability_list::{BackendNotUsable, NotUsableReason};

/// What the operator can do about `backend` being not usable for `reason` (CAP-002).
#[must_use]
pub fn backend_hint(backend: &str, reason: &BackendNotUsable) -> &'static str {
    match (backend, reason) {
        (_, BackendNotUsable::NO_DEVICE) => {
            "No GPU of this backend's vendor is in the device; use another backend."
        }
        ("cuda", BackendNotUsable::NO_DRIVER) => {
            "Install the NVIDIA driver on the device (on Pop!_OS: sudo apt install \
             system76-driver-nvidia), then restart it."
        }
        ("rocm", BackendNotUsable::NO_DRIVER) => {
            "Install the AMD ROCm driver on the device so that /dev/kfd is present."
        }
        ("vulkan", BackendNotUsable::NO_DRIVER) => {
            "Install the Vulkan driver of the GPU on the device (for example the package \
             mesa-vulkan-drivers, or the NVIDIA driver)."
        }
        (_, BackendNotUsable::NO_PERMISSION) => {
            "Add the daemon's user to the render and video groups on the device, then sign in \
             again: sudo usermod -aG render,video <user>."
        }
        (_, BackendNotUsable::NOT_SUPPORTED) => {
            "This backend does not exist on the device's system or processor; use another."
        }
        (_, BackendNotUsable::NO_DRIVER) => "Install the driver of the GPU on the device.",
        (_, BackendNotUsable::FAILED | BackendNotUsable::Other(_)) => {
            "Update the hub and this command line to the daemon's version to see why."
        }
    }
}

/// Whether the hint of `reason` is worth a note: the device has the hardware, and the operator
/// can repair it.
#[must_use]
pub const fn backend_repairable(reason: &BackendNotUsable) -> bool {
    !matches!(
        reason,
        BackendNotUsable::NO_DEVICE | BackendNotUsable::NOT_SUPPORTED
    )
}

/// What the operator can do about `tool` being not usable for `reason`.
#[must_use]
pub fn hint(tool: &str, reason: &NotUsableReason) -> &'static str {
    match (tool, reason) {
        ("xcodebuild", NotUsableReason::EXITED | NotUsableReason::NO_VERSION) => {
            "Install Xcode from the App Store, open it once, then run \
             sudo xcode-select --switch /Applications/Xcode.app on the device."
        }
        ("rustc" | "cargo" | "cargo-nextest", NotUsableReason::EXITED) => {
            "Install a default Rust toolchain on the device: rustup default stable. The daemon \
             never lets rustup download one."
        }
        ("claude" | "codex", NotUsableReason::EXITED) => {
            "Update or reinstall it on the device; the daemon runs it with an empty home folder, \
             so it must print its version without a sign-in."
        }
        (_, NotUsableReason::TIMED_OUT) => {
            "Its version check ran past 5 seconds: run it with --version on the device to see \
             what it waits for."
        }
        (_, NotUsableReason::EXITED) => {
            "It could not start or exited with a failure: run it with --version on the device \
             to see the error, then repair or reinstall it."
        }
        (_, NotUsableReason::NO_VERSION) => {
            "It printed no version the daemon reads: it may be another program of the same name, \
             or a newer output format that a newer daemon reads."
        }
        (_, NotUsableReason::FAILED | NotUsableReason::Other(_)) => {
            "Update the hub and this command line to the daemon's version to see why."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cap_024_each_reason_has_a_sentence_and_some_tools_a_sentence_of_their_own() {
        let reasons = [
            NotUsableReason::TIMED_OUT,
            NotUsableReason::EXITED,
            NotUsableReason::NO_VERSION,
            NotUsableReason::FAILED,
            NotUsableReason::Other("denied".to_owned()),
        ];
        for reason in &reasons {
            let sentence = hint("bun", reason);
            assert!(sentence.ends_with('.'), "{sentence}");
        }
        assert!(hint("xcodebuild", &NotUsableReason::EXITED).contains("xcode-select"));
        assert!(hint("cargo", &NotUsableReason::EXITED).contains("rustup default"));
        assert!(hint("codex", &NotUsableReason::EXITED).contains("empty home"));
        assert_eq!(
            hint("cargo", &NotUsableReason::TIMED_OUT),
            hint("bun", &NotUsableReason::TIMED_OUT)
        );
        assert!(backend_hint("rocm", &BackendNotUsable::NO_PERMISSION).contains("render"));
        assert!(backend_hint("cuda", &BackendNotUsable::NO_DRIVER).contains("NVIDIA driver"));
        assert!(!backend_repairable(&BackendNotUsable::NOT_SUPPORTED));
        assert!(backend_repairable(&BackendNotUsable::NO_DRIVER));
        assert_eq!(
            hint("bun", &NotUsableReason::Other("denied".to_owned())),
            hint("bun", &NotUsableReason::FAILED),
            "a reason this version does not know"
        );
    }
}
