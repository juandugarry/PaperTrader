//! User-approved local key storage. No OS credential store is accessed.
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};

pub fn configured(path: &Path) -> Result<bool, String> {
    path.try_exists()
        .map_err(|_| "Unable to check the local API key file.".into())
}
pub fn load(path: &Path) -> Result<Option<String>, String> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => {
            return Err("Unable to read the local API key. Save it again in Settings.".into())
        }
    };
    let mut bytes = Vec::new();
    file.take(161)
        .read_to_end(&mut bytes)
        .map_err(|_| "Unable to read the local API key.")?;
    let key = String::from_utf8(bytes)
        .map_err(|_| "Invalid local API key. Save it again in Settings.")?;
    crate::market::validate_key(&key)?;
    Ok(Some(key))
}
pub fn save(path: &Path, key: &str) -> Result<(), String> {
    let key = crate::market::validate_key(key)?;
    let temporary = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let result = (|| {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary)
            .map_err(|_| "Unable to save the local API key.")?;
        file.write_all(key.as_bytes())
            .map_err(|_| "Unable to save the local API key.")?;
        file.sync_all()
            .map_err(|_| "Unable to save the local API key.")?;
        drop(file);
        fs::rename(&temporary, path).map_err(|_| "Unable to replace the local API key.")?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
pub fn remove(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("Unable to remove the local API key file.".into()),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_key_saves_replaces_and_removes_without_system_credentials() {
        let dir =
            std::env::temp_dir().join(format!("papertrader-credentials-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("key.txt");
        remove(&path).unwrap();
        assert!(!configured(&path).unwrap());
        assert!(load(&path).unwrap().is_none());
        save(&path, "example_key_123").unwrap();
        assert!(configured(&path).unwrap());
        assert_eq!(load(&path).unwrap().as_deref(), Some("example_key_123"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        save(&path, "replacement_key_456").unwrap();
        assert_eq!(load(&path).unwrap().as_deref(), Some("replacement_key_456"));
        assert!(save(&path, "bad?key").is_err());
        assert_eq!(load(&path).unwrap().as_deref(), Some("replacement_key_456"));
        remove(&path).unwrap();
        remove(&path).unwrap();
        assert!(load(&path).unwrap().is_none());
        let _ = fs::remove_dir(dir);
    }
}
