//! Current-user PATH registration. Never changes the machine PATH or expands its variables.
use std::{io, path::Path, sync::Mutex};
use winreg::{
    RegKey,
    enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_EXPAND_SZ, REG_SZ},
    types::{FromRegValue, ToRegValue},
};

static CHANGING: Mutex<()> = Mutex::new(());

fn same(left: &str, right: &str) -> bool {
    left.trim_matches('"')
        .trim_end_matches(['\\', '/'])
        .eq_ignore_ascii_case(right.trim_end_matches(['\\', '/']))
}

fn contains(value: &str, folder: &str) -> bool {
    value.split(';').any(|entry| same(entry, folder))
}

fn edited(value: &str, folder: &str, install: bool) -> String {
    if install {
        if contains(value, folder) {
            value.to_owned()
        } else if value.is_empty() {
            folder.to_owned()
        } else {
            format!("{value};{folder}")
        }
    } else {
        value
            .split(';')
            .filter(|entry| !same(entry, folder))
            .collect::<Vec<_>>()
            .join(";")
    }
}

fn read(key: &RegKey) -> io::Result<(String, winreg::enums::RegType)> {
    match key.get_raw_value("Path") {
        Ok(raw) => {
            if !matches!(raw.vtype, REG_SZ | REG_EXPAND_SZ) {
                return Err(io::ErrorKind::InvalidData.into());
            }
            let value = String::from_reg_value(&raw)?;
            if value.to_reg_value().bytes != raw.bytes {
                return Err(io::ErrorKind::InvalidData.into());
            }
            Ok((value, raw.vtype))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok((String::new(), REG_EXPAND_SZ)),
        Err(error) => Err(error),
    }
}

fn text(folder: &Path) -> io::Result<&str> {
    folder
        .to_str()
        .filter(|text| folder.is_absolute() && !text.contains([';', '%', '"', '\0']))
        .ok_or_else(|| io::ErrorKind::InvalidInput.into())
}

/// Whether the literal folder is registered in this user's persistent PATH.
/// # Errors
/// The registry cannot be read or the folder cannot be represented in PATH.
pub fn registered(folder: &Path) -> io::Result<bool> {
    let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("Environment", KEY_READ)?;
    Ok(contains(&read(&key)?.0, text(folder)?))
}

fn change(key: &RegKey, folder: &str, install: bool) -> io::Result<()> {
    let (before, kind) = read(key)?;
    let after = edited(&before, folder, install);
    if after == before {
        return Ok(());
    }
    let mut value = after.to_reg_value();
    value.vtype = kind;
    key.set_raw_value("Path", &value)
}

/// Add or remove only the supplied folder. Preserve unrelated entries and registry string type.
/// # Errors
/// The folder is invalid or Windows refuses the registry write.
pub fn register(folder: &Path, install: bool) -> io::Result<()> {
    let _guard = CHANGING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)?;
    change(&key, text(folder)?, install)?;
    crate::native::environment::changed();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registration_preserves_variables_empty_entries_and_other_programs() {
        let before = r"%USERPROFILE%\bin;;C:\Other";
        let folder = r"C:\Users\User\Anima.ing";
        let added = edited(before, folder, true);
        assert_eq!(edited(&added, folder, true), added);
        assert_eq!(edited(&added, folder, false), before);
        assert!(contains(r#"C:\Other;"C:\USERS\USER\ANIMA.ING\""#, folder));
        assert_eq!(
            edited(r"C:\Anima.ing-other", r"C:\Anima.ing", false),
            r"C:\Anima.ing-other"
        );
    }
    #[test]
    fn registry_edit_preserves_expandable_values_and_round_trips() {
        let name = format!(r"Software\AnimaTest\Path-{}", std::process::id());
        let root = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = root.create_subkey(&name).unwrap();
        let mut original = r"%USERPROFILE%\bin;C:\Other".to_reg_value();
        original.vtype = REG_EXPAND_SZ;
        key.set_raw_value("Path", &original).unwrap();
        change(&key, r"C:\Anima.ing", true).unwrap();
        assert_eq!(read(&key).unwrap().1, REG_EXPAND_SZ);
        change(&key, r"C:\Anima.ing", false).unwrap();
        assert_eq!(key.get_raw_value("Path").unwrap(), original);
        drop(key);
        root.delete_subkey_all(name).unwrap();
    }
}
