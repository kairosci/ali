use crate::{
    embedding,
    settings::{Config, Locale},
};
use anyhow::{Context, Result, bail};
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};
use directories::ProjectDirs;
use fs2::FileExt;
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

const MAGIC: &[u8; 4] = b"ALI1";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;
const AAD: &[u8] = b"ALI encrypted local memory v1";

#[derive(Debug, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
struct Vault {
    version: u8,
    entries: Vec<Entry>,
}

#[derive(Debug, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
struct Entry {
    id: u64,
    text: String,
    #[serde(default)]
    source_agent: Option<String>,
    embedding_model: String,
    embedding: Vec<f32>,
}

pub enum Action {
    Init,
    Add { agent_id: Option<String> },
    Search { agent_id: Option<String> },
    List,
    Clear,
}

pub fn default_data_dir(config: &Config) -> Result<PathBuf> {
    let dirs = ProjectDirs::from(
        &config.storage.qualifier,
        &config.storage.organization,
        &config.storage.application,
    )
    .context("no platform data directory is available")?;
    Ok(dirs.data_local_dir().to_owned())
}

pub async fn run(
    action: Action,
    config: &Config,
    locale: &Locale,
    override_dir: Option<&Path>,
) -> Result<()> {
    let root = match override_dir {
        Some(path) => path.to_owned(),
        None => default_data_dir(config)?,
    };
    match action {
        Action::Init => initialize(&root, config, locale),
        Action::Add { agent_id } => add(&root, config, locale, agent_id.as_deref()).await,
        Action::Search { agent_id } => search(&root, config, locale, agent_id.as_deref()).await,
        Action::List => list(&root, config, locale),
        Action::Clear => clear(&root, config, locale),
    }
}

fn ensure_private_dir(root: &Path) -> Result<()> {
    if !root.exists() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            let mut builder = fs::DirBuilder::new();
            builder.recursive(true).mode(0o700).create(root)?;
        }
        #[cfg(not(unix))]
        fs::create_dir_all(root)?;
    }
    let metadata = fs::symlink_metadata(root).context("cannot inspect memory directory")?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        bail!("memory directory must be a real directory, not a symlink");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            bail!("memory directory permissions must deny access to group and other users");
        }
    }
    Ok(())
}

fn lock(root: &Path) -> Result<File> {
    let path = root.join("memory.lock");
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW).mode(0o600);
    }
    let file = options.open(&path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        bail!("memory lock must be a regular file");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    file.try_lock_exclusive()
        .context("local memory is busy in another ALI process")?;
    Ok(file)
}

fn read_password(prompt: &str) -> Result<Zeroizing<String>> {
    let value = rpassword::prompt_password(prompt)
        .context("could not read the vault passphrase securely")?;
    if value.chars().count() < 12 {
        bail!("vault passphrase must contain at least 12 characters");
    }
    Ok(Zeroizing::new(value))
}

fn derive_key(password: &[u8], salt: &[u8], config: &Config) -> Result<Zeroizing<[u8; 32]>> {
    let params = Params::new(
        config.security.argon2_memory_kib,
        config.security.argon2_iterations,
        config.security.argon2_lanes,
        Some(32),
    )
    .map_err(|error| anyhow::anyhow!("invalid Argon2 parameters: {error}"))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0u8; 32]);
    argon
        .hash_password_into(password, salt, key.as_mut())
        .map_err(|error| anyhow::anyhow!("Argon2 key derivation failed: {error}"))?;
    Ok(key)
}

fn encrypt(vault: &Vault, password: &[u8], config: &Config) -> Result<Vec<u8>> {
    let mut salt = [0u8; SALT_LEN];
    let mut nonce = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce);
    let key = derive_key(password, &salt, config)?;
    let cipher = XChaCha20Poly1305::new_from_slice(key.as_ref())
        .map_err(|_| anyhow::anyhow!("invalid encryption key length"))?;
    let plaintext = Zeroizing::new(serde_json::to_vec(vault)?);
    let ciphertext = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            chacha20poly1305::aead::Payload {
                msg: plaintext.as_slice(),
                aad: AAD,
            },
        )
        .map_err(|_| anyhow::anyhow!("memory encryption failed"))?;
    let mut output = Vec::with_capacity(MAGIC.len() + SALT_LEN + NONCE_LEN + ciphertext.len());
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&salt);
    output.extend_from_slice(&nonce);
    output.extend_from_slice(&ciphertext);
    Ok(output)
}

fn decrypt(bytes: &[u8], password: &[u8], config: &Config) -> Result<Vault> {
    if bytes.len() < MAGIC.len() + SALT_LEN + NONCE_LEN + 16 || !bytes.starts_with(MAGIC) {
        bail!("memory vault has an invalid or unsupported format");
    }
    let salt_start = MAGIC.len();
    let nonce_start = salt_start + SALT_LEN;
    let cipher_start = nonce_start + NONCE_LEN;
    let key = derive_key(password, &bytes[salt_start..nonce_start], config)?;
    let cipher = XChaCha20Poly1305::new_from_slice(key.as_ref())
        .map_err(|_| anyhow::anyhow!("invalid encryption key length"))?;
    let plaintext = Zeroizing::new(cipher.decrypt(
        XNonce::from_slice(&bytes[nonce_start..cipher_start]),
        chacha20poly1305::aead::Payload { msg: &bytes[cipher_start..], aad: AAD },
    ).map_err(|_| anyhow::anyhow!("vault authentication failed; passphrase may be incorrect or data may be damaged"))?);
    let vault: Vault =
        serde_json::from_slice(plaintext.as_slice()).context("decrypted memory data is invalid")?;
    if vault.version != 1 {
        bail!("memory vault data version is unsupported");
    }
    Ok(vault)
}

fn save(root: &Path, vault: &Vault, password: &[u8], config: &Config) -> Result<()> {
    let bytes = encrypt(vault, password, config)?;
    let mut temp = tempfile::NamedTempFile::new_in(root)?;
    temp.write_all(&bytes)?;
    temp.as_file().sync_all()?;
    let target = root.join("memory.vault");
    if fs::symlink_metadata(&target).is_ok_and(|meta| meta.file_type().is_symlink()) {
        bail!("memory vault must not be a symlink");
    }
    temp.persist(&target)
        .map_err(|error| error.error)
        .context("atomic memory-vault write failed")?;
    Ok(())
}

fn load(root: &Path, password: &[u8], config: &Config) -> Result<Vault> {
    let path = root.join("memory.vault");
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options
        .open(&path)
        .context("memory is not initialized; run `ali memory init`")?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        bail!("memory vault must be a regular file, not a symlink");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            bail!("memory vault permissions must deny access to group and other users");
        }
    }
    if metadata.len() > config.security.max_document_bytes.saturating_mul(100) {
        bail!("memory vault exceeds the configured storage size limit");
    }
    let mut bytes = Vec::new();
    file.take(config.security.max_document_bytes.saturating_mul(100) + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > config.security.max_document_bytes.saturating_mul(100) {
        bail!("memory vault exceeds the configured storage size limit");
    }
    decrypt(&bytes, password, config)
}

fn initialize(root: &Path, config: &Config, locale: &Locale) -> Result<()> {
    ensure_private_dir(root)?;
    let _lock = lock(root)?;
    let path = root.join("memory.vault");
    if path.exists() {
        bail!("memory vault already exists");
    }
    let first = read_password(locale.message("vault_passphrase")?)?;
    let second = read_password(locale.message("vault_confirm_passphrase")?)?;
    if first.as_str() != second.as_str() {
        bail!("vault passphrases do not match");
    }
    save(
        root,
        &Vault {
            version: 1,
            entries: vec![],
        },
        first.as_bytes(),
        config,
    )?;
    println!("{}", locale.message("memory_created")?);
    Ok(())
}

async fn add(root: &Path, config: &Config, locale: &Locale, agent_id: Option<&str>) -> Result<()> {
    ensure_private_dir(root)?;
    validate_agent_id(agent_id, config)?;
    eprintln!("{}", locale.message("memory_input")?);
    let mut text = Zeroizing::new(String::new());
    io::stdin()
        .take(config.security.max_document_bytes + 1)
        .read_to_string(&mut text)?;
    if text.trim().is_empty() {
        bail!("memory text is empty");
    }
    if text.len() as u64 > config.security.max_document_bytes {
        bail!("{}", locale.message("document_too_large")?);
    }
    let password = read_password(locale.message("vault_passphrase")?)?;
    let _lock = lock(root)?;
    let mut vault = load(root, password.as_bytes(), config)?;
    let vector = embedding::embed(&text, config).await?;
    let id = vault
        .entries
        .iter()
        .map(|entry| entry.id)
        .max()
        .map_or(1, |max| max.saturating_add(1));
    vault.entries.push(Entry {
        id,
        text: std::mem::take(&mut *text),
        source_agent: agent_id.map(str::to_owned),
        embedding_model: config.embedding.model.clone(),
        embedding: vector,
    });
    save(root, &vault, password.as_bytes(), config)?;
    Ok(())
}

fn list(root: &Path, config: &Config, locale: &Locale) -> Result<()> {
    ensure_private_dir(root)?;
    let password = read_password(locale.message("vault_passphrase")?)?;
    let _lock = lock(root)?;
    let vault = load(root, password.as_bytes(), config)?;
    for entry in &vault.entries {
        println!(
            "{}\t{}\t{}",
            entry.id,
            entry
                .source_agent
                .as_deref()
                .map(str::to_owned)
                .unwrap_or(locale.message("memory_shared_label")?.to_owned()),
            entry.text
        );
    }
    Ok(())
}

async fn search(
    root: &Path,
    config: &Config,
    locale: &Locale,
    agent_id: Option<&str>,
) -> Result<()> {
    ensure_private_dir(root)?;
    validate_agent_id(agent_id, config)?;
    eprintln!("{}", locale.message("memory_query")?);
    let mut query = Zeroizing::new(String::new());
    io::stdin()
        .take(config.security.max_document_bytes + 1)
        .read_to_string(&mut query)?;
    if query.trim().is_empty() {
        bail!("memory search query is empty");
    }
    let password = read_password(locale.message("vault_passphrase")?)?;
    let _lock = lock(root)?;
    let vault = load(root, password.as_bytes(), config)?;
    let query_vector = embedding::embed(&query, config).await?;
    let mut ranked: Vec<(f32, &Entry)> = vault
        .entries
        .iter()
        .filter(|entry| {
            (agent_id.is_none()
                || entry.source_agent.is_none()
                || entry.source_agent.as_deref() == agent_id)
                && entry.embedding_model == config.embedding.model
                && entry.embedding.len() == query_vector.len()
        })
        .map(|entry| (cosine(&query_vector, &entry.embedding), entry))
        .collect();
    ranked.sort_by(|left, right| right.0.total_cmp(&left.0));
    for (score, entry) in ranked.into_iter().take(5) {
        println!(
            "{score:.4}\t{}\t{}\t{}",
            entry.id,
            entry
                .source_agent
                .as_deref()
                .map(str::to_owned)
                .unwrap_or(locale.message("memory_shared_label")?.to_owned()),
            entry.text
        );
    }
    Ok(())
}

fn validate_agent_id(agent_id: Option<&str>, config: &Config) -> Result<()> {
    if let Some(agent_id) = agent_id
        && !config.agents.iter().any(|agent| agent.id == agent_id)
    {
        bail!("memory agent identifier is not present in config/agents.toml");
    }
    Ok(())
}

fn clear(root: &Path, config: &Config, locale: &Locale) -> Result<()> {
    ensure_private_dir(root)?;
    let password = read_password(locale.message("vault_passphrase")?)?;
    let _lock = lock(root)?;
    let _ = load(root, password.as_bytes(), config)?;
    fs::remove_file(root.join("memory.vault"))?;
    println!("{}", locale.message("memory_cleared")?);
    Ok(())
}

fn cosine(left: &[f32], right: &[f32]) -> f32 {
    let dot: f32 = left.iter().zip(right).map(|(a, b)| a * b).sum();
    let left_norm = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let right_norm = right.iter().map(|value| value * value).sum::<f32>().sqrt();
    if left_norm == 0.0 || right_norm == 0.0 {
        0.0
    } else {
        dot / (left_norm * right_norm)
    }
}
