use std::env;
use std::path::PathBuf;

const ALLWRIGHT_HOME_ENV_VAR: &str = "ALLWRIGHT_HOME";

pub(crate) fn allwright_home() -> Result<PathBuf, String> {
    resolve_allwright_home(
        env::var(ALLWRIGHT_HOME_ENV_VAR).ok().as_deref(),
        env::var("HOME").ok().as_deref(),
        env::var("USERPROFILE").ok().as_deref(),
        env::var("HOMEDRIVE").ok().as_deref(),
        env::var("HOMEPATH").ok().as_deref(),
        cfg!(windows),
    )
}

fn resolve_allwright_home(
    configured: Option<&str>,
    home: Option<&str>,
    user_profile: Option<&str>,
    home_drive: Option<&str>,
    home_path: Option<&str>,
    windows: bool,
) -> Result<PathBuf, String> {
    if let Some(configured) = non_empty(configured) {
        return Ok(PathBuf::from(configured));
    }

    let user_home = if windows {
        non_empty(user_profile)
            .map(PathBuf::from)
            .or_else(|| windows_home_from_drive_and_path(home_drive, home_path))
            .or_else(|| non_empty(home).map(PathBuf::from))
    } else {
        non_empty(home).map(PathBuf::from)
    };

    user_home
        .map(|path| path.join(".allwright"))
        .ok_or_else(|| {
            "could not determine the user home directory; set ALLWRIGHT_HOME explicitly".to_string()
        })
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn windows_home_from_drive_and_path(
    home_drive: Option<&str>,
    home_path: Option<&str>,
) -> Option<PathBuf> {
    let drive = non_empty(home_drive)?;
    let path = non_empty(home_path)?;
    Some(PathBuf::from(format!("{drive}{path}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_allwright_home_takes_precedence() {
        assert_eq!(
            resolve_allwright_home(
                Some(" C:\\allwright "),
                Some("/home/tester"),
                Some("C:\\Users\\tester"),
                None,
                None,
                true,
            )
            .unwrap(),
            PathBuf::from("C:\\allwright")
        );
    }

    #[test]
    fn windows_defaults_to_user_profile() {
        assert_eq!(
            resolve_allwright_home(None, None, Some("C:\\Users\\tester"), None, None, true,)
                .unwrap(),
            PathBuf::from("C:\\Users\\tester").join(".allwright")
        );
    }

    #[test]
    fn windows_supports_drive_and_path_fallback() {
        assert_eq!(
            resolve_allwright_home(None, None, None, Some("C:"), Some("\\Users\\tester"), true)
                .unwrap(),
            PathBuf::from("C:\\Users\\tester").join(".allwright")
        );
    }

    #[test]
    fn unix_defaults_to_home() {
        assert_eq!(
            resolve_allwright_home(
                None,
                Some("/home/tester"),
                Some("C:\\Users\\tester"),
                None,
                None,
                false,
            )
            .unwrap(),
            PathBuf::from("/home/tester/.allwright")
        );
    }
}
