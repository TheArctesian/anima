//! Find installed MSVC and SDK tools when the app was opened from the Start menu.
use super::Environment;
use crate::os_program::{self, AfterLimit, Limit, Stdout};
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

pub fn compiler(mut environment: Environment) -> Environment {
    let Some((vc, sdk, version)) = installed() else {
        return environment;
    };
    let sdk_include = sdk.join("Include").join(&version);
    let includes = [
        vc.join("include"),
        sdk_include.join("ucrt"),
        sdk_include.join("shared"),
        sdk_include.join("um"),
        sdk_include.join("winrt"),
        sdk_include.join("cppwinrt"),
    ];
    let libs = [
        vc.join("lib/x64"),
        sdk.join("Lib").join(&version).join("ucrt/x64"),
        sdk.join("Lib").join(&version).join("um/x64"),
    ];
    for (name, paths) in [("INCLUDE", includes.as_slice()), ("LIB", libs.as_slice())] {
        let Ok(value) = std::env::join_paths(paths) else {
            return environment;
        };
        environment
            .passed
            .retain(|(key, _)| !key.to_string_lossy().eq_ignore_ascii_case(name));
        environment.passed.push((name.into(), value));
    }
    environment
        .dirs
        .insert(0, sdk.join("bin").join(&version).join("x64"));
    environment.dirs.insert(0, vc.join("bin/Hostx64/x64"));
    environment
}

fn installed() -> Option<(PathBuf, PathBuf, String)> {
    let programs = PathBuf::from(std::env::var_os("ProgramFiles(x86)")?);
    if !programs.is_absolute() {
        return None;
    }
    let locator = programs.join("Microsoft Visual Studio/Installer/vswhere.exe");
    anima_windows::files::read(&locator, false).ok()?;
    let bytes = os_program::run(
        locator.to_str()?,
        &[
            "-latest",
            "-products",
            "*",
            "-requires",
            "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
            "-property",
            "installationPath",
            "-utf8",
        ]
        .map(OsStr::new),
        Limit {
            patience: Duration::from_secs(5),
            after: AfterLimit::Kill,
        },
        Stdout::Capture { max_bytes: 8192 },
    )
    .ok()?;
    let text = String::from_utf8(bytes).ok()?;
    let installation = Path::new(text.trim());
    if !installation.is_absolute() {
        return None;
    }
    let version = fs::read_to_string(
        installation.join("VC/Auxiliary/Build/Microsoft.VCToolsVersion.default.txt"),
    )
    .ok()?;
    if !version_name(version.trim()) {
        return None;
    }
    let vc = installation.join("VC/Tools/MSVC").join(version.trim());
    if !vc.join("bin/Hostx64/x64/link.exe").is_file() {
        return None;
    }
    let sdk = programs.join("Windows Kits/10");
    let version = sdk_version(&sdk)?;
    Some((vc, sdk, version))
}

fn version_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 40
        && value.split('.').all(|part| part.parse::<u32>().is_ok())
}

fn sdk_version(sdk: &Path) -> Option<String> {
    let mut versions: Vec<String> = fs::read_dir(sdk.join("Include"))
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| version_name(name))
        .filter(|name| {
            sdk.join("bin").join(name).join("x64/rc.exe").is_file()
                && sdk
                    .join("Lib")
                    .join(name)
                    .join("um/x64/kernel32.lib")
                    .is_file()
        })
        .collect();
    versions.sort_by_cached_key(|name| {
        name.split('.')
            .filter_map(|part| part.parse::<u32>().ok())
            .collect::<Vec<_>>()
    });
    versions.pop()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_numeric_version_components_can_enter_a_tool_path() {
        for bad in [
            "",
            "..",
            "../outside",
            "14.42/other",
            "14. 42",
            "14.42;other",
            "14..42",
        ] {
            assert!(!version_name(bad), "{bad}");
        }
        assert!(version_name("14.44.35207"));
    }
}
