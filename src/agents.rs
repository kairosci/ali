use crate::settings::{Config, Locale};
use anyhow::{Context, Result};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

pub fn list(config: &Config, locale: &Locale, language: &str) -> Result<()> {
    println!("{}", locale.message("agents_list_header")?);
    for agent in &config.agents {
        let executable = resolve(&agent.executable)?;
        let label = agent
            .labels
            .get(language)
            .context("validated agent label is missing")?;
        match executable {
            Some(path) => println!(
                "{}\t{}\t{}\t{}",
                agent.id,
                label,
                locale.message("agent_found")?,
                path.display()
            ),
            None => println!(
                "{}\t{}\t{}",
                agent.id,
                label,
                locale.message("agent_not_found")?
            ),
        }
    }
    Ok(())
}

fn resolve(executable: &str) -> Result<Option<PathBuf>> {
    let candidate = Path::new(executable);
    if candidate.components().count() > 1 || candidate.is_absolute() {
        return Ok(is_executable(candidate).then(|| candidate.to_owned()));
    }
    let path = env::var_os("PATH")
        .context("PATH is unavailable; configure absolute agent executable paths")?;
    for directory in env::split_paths(&path) {
        let candidate = directory.join(executable);
        if is_executable(&candidate) {
            return Ok(Some(candidate));
        }
        #[cfg(windows)]
        for extension in ["exe", "cmd", "bat"] {
            let with_extension = directory.join(format!("{executable}.{extension}"));
            if is_executable(&with_extension) {
                return Ok(Some(with_extension));
            }
        }
    }
    Ok(None)
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(windows)]
    {
        path.extension().is_some_and(|extension| {
            extension.eq_ignore_ascii_case("exe")
                || extension.eq_ignore_ascii_case("cmd")
                || extension.eq_ignore_ascii_case("bat")
        })
    }
    #[cfg(not(any(unix, windows)))]
    {
        true
    }
}
