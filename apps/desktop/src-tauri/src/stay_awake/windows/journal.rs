//! A locked, durable record precedes activation; recovery never edits the original plan.
use anima_windows::{files, power::scheme::Scheme};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::Path,
};

pub struct Journal {
    file: File,
    previous: Scheme,
    temporary: Scheme,
}
impl Journal {
    pub fn start(marker: &Path) -> io::Result<Self> {
        restore(marker)?;
        files::directory(marker.parent().ok_or(io::ErrorKind::InvalidInput)?)?;
        let mut file = files::create_new(marker)?;
        file.try_lock().map_err(io::Error::other)?;
        let previous = Scheme::active()?;
        let temporary = match previous.without_lid_sleep() {
            Ok(scheme) => scheme,
            Err(error) => {
                drop(file);
                fs::remove_file(marker)?;
                return Err(error);
            }
        };
        let result = writeln!(file, "{}\n{}", previous.text(), temporary.text())
            .and_then(|()| file.sync_all());
        if let Err(error) = result {
            let _ = temporary.remove();
            drop(file);
            let _ = fs::remove_file(marker);
            return Err(error);
        }
        Ok(Self {
            file,
            previous,
            temporary,
        })
    }

    pub fn activate(&self) -> io::Result<()> {
        self.temporary.activate()
    }

    pub fn restore(&self) -> io::Result<()> {
        restore_schemes(self.previous, self.temporary)?;
        // Keep the file locked until the caller has removed the marker.
        self.file.sync_all()
    }
}

fn restore_schemes(previous: Scheme, temporary: Scheme) -> io::Result<()> {
    if previous == temporary {
        return Err(io::ErrorKind::InvalidData.into());
    }
    if Scheme::active()? == temporary {
        previous.activate()?;
    }
    // A user selecting a different plan wins. Only our inactive copy is deleted.
    match temporary.remove() {
        Ok(()) => Ok(()),
        Err(error) if error.raw_os_error() == Some(2) => Ok(()),
        Err(error) => Err(error),
    }
}

pub fn restore(marker: &Path) -> io::Result<()> {
    let mut file = match files::read(marker, true) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    file.try_lock().map_err(io::Error::other)?;
    let mut text = String::new();
    (&mut file).take(128).read_to_string(&mut text)?;
    let parts: Vec<_> = text.lines().collect();
    match parts.as_slice() {
        [] => {} // A stopped creation never activated a plan.
        [previous, temporary] => {
            restore_schemes(Scheme::parse(previous)?, Scheme::parse(temporary)?)?;
        }
        _ => return Err(io::ErrorKind::InvalidData.into()),
    }
    fs::remove_file(marker)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Reset {
        original: Scheme,
        marker: std::path::PathBuf,
        extra: Option<Scheme>,
    }
    impl Drop for Reset {
        fn drop(&mut self) {
            let _ = restore(&self.marker);
            let _ = self.original.activate();
            if let Some(extra) = self.extra {
                let _ = extra.remove();
            }
        }
    }

    #[test]
    fn a_lid_hold_restores_after_release_and_recovery_and_preserves_a_later_choice() {
        let folder = tempfile::tempdir().unwrap();
        files::protect(folder.path()).unwrap();
        let marker = folder.path().join("lid");
        let original = Scheme::active().unwrap();
        let mut reset = Reset {
            original,
            marker: marker.clone(),
            extra: None,
        };
        let journal = Journal::start(&marker).unwrap();
        journal.activate().unwrap();
        assert_ne!(Scheme::active().unwrap(), original);
        assert!(restore(&marker).is_err(), "a live guard owns its journal");
        journal.restore().unwrap();
        assert_eq!(Scheme::active().unwrap(), original);
        fs::remove_file(&marker).unwrap();
        drop(journal);

        let journal = Journal::start(&marker).unwrap();
        journal.activate().unwrap();
        drop(journal); // The recorded plan survives an interrupted guard.
        restore(&marker).unwrap();
        assert_eq!(Scheme::active().unwrap(), original);
        assert!(!marker.exists());

        let journal = Journal::start(&marker).unwrap();
        journal.activate().unwrap();
        let chosen = original.without_lid_sleep().unwrap();
        reset.extra = Some(chosen);
        chosen.activate().unwrap();
        drop(journal);
        restore(&marker).unwrap();
        assert_eq!(Scheme::active().unwrap(), chosen);
    }
}
