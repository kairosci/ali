use crate::{
    embedding,
    settings::{Config, Locale},
};
use anyhow::{Context, Result, bail};
use regex::Regex;
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::Path,
};
use zeroize::Zeroizing;

pub struct ScanResult {
    pub redacted: String,
    pub findings: BTreeMap<String, usize>,
}

pub struct ScanReport {
    pub output_path: std::path::PathBuf,
    pub redacted: String,
    pub findings: BTreeMap<String, usize>,
}

pub struct SemanticCue {
    pub segment: usize,
    pub category: String,
    pub similarity: f32,
}

pub struct SemanticReport {
    pub segments_analyzed: usize,
    pub cues: Vec<SemanticCue>,
}

pub struct ReviewReport {
    pub output_path: std::path::PathBuf,
    pub redacted: String,
    pub findings: BTreeMap<String, usize>,
    pub semantic: SemanticReport,
}

pub fn format_semantic_report(report: &SemanticReport, locale: &Locale) -> Result<String> {
    if report.cues.is_empty() {
        return Ok(locale
            .message("privacy_analysis_no_cues")?
            .replace("{segments}", &report.segments_analyzed.to_string()));
    }
    let mut details = Vec::new();
    let mut categories = std::collections::BTreeSet::new();
    for cue in &report.cues {
        let label = locale.message(&format!("privacy_cue_{}", cue.category))?;
        categories.insert(label);
        details.push(
            locale
                .message("privacy_cue_detail")?
                .replace("{category}", label)
                .replace("{segment}", &cue.segment.to_string())
                .replace("{similarity}", &format!("{:.3}", cue.similarity)),
        );
    }
    let summary = locale
        .message("privacy_analysis_complete")?
        .replace("{segments}", &report.segments_analyzed.to_string())
        .replace("{count}", &report.cues.len().to_string())
        .replace(
            "{categories}",
            &categories.into_iter().collect::<Vec<_>>().join(", "),
        );
    Ok(format!("{summary}\n{}", details.join("\n")))
}

pub fn redact(text: &str, config: &Config) -> Result<ScanResult> {
    let mut redacted = text.to_owned();
    let mut findings = BTreeMap::new();
    for detector in &config.sensitivity {
        let pattern = Regex::new(&detector.regex)
            .with_context(|| format!("invalid detector {}", detector.category))?;
        let mut count = 0usize;
        let replacement = pattern.replace_all(&redacted, |capture: &regex::Captures<'_>| {
            let matched = capture.get(0).map_or("", |value| value.as_str());
            if detector.category == "payment_card_candidate" && !passes_luhn(matched) {
                return matched.to_owned();
            }
            count = count.saturating_add(1);
            format!("[REDACTED:{}]", detector.category)
        });
        redacted = replacement.into_owned();
        if count > 0 {
            findings.insert(detector.category.clone(), count);
        }
    }
    Ok(ScanResult { redacted, findings })
}

pub fn scan_file(input: &Path, output: &Path, config: &Config) -> Result<ScanReport> {
    let text = read_selected_text(input, config)?;
    let result = redact(&text, config)?;
    let destination = write_candidate(output, &result.redacted)?;
    Ok(ScanReport {
        output_path: destination,
        redacted: result.redacted,
        findings: result.findings,
    })
}

pub async fn review_file(input: &Path, output: &Path, config: &Config) -> Result<ReviewReport> {
    let text = read_selected_text(input, config)?;
    let result = redact(&text, config)?;
    let redacted = Zeroizing::new(result.redacted);
    let semantic = analyze_text(&redacted, config).await?;
    let destination = write_candidate(output, &redacted)?;
    Ok(ReviewReport {
        output_path: destination,
        redacted: redacted.to_string(),
        findings: result.findings,
        semantic,
    })
}

fn write_candidate(output: &Path, contents: &str) -> Result<std::path::PathBuf> {
    let absolute_output = if output.is_absolute() {
        output.to_owned()
    } else {
        std::env::current_dir()?.join(output)
    };
    let parent = absolute_output
        .parent()
        .context("output path has no parent")?;
    let parent = fs::canonicalize(parent).context("output directory does not exist")?;
    let name = absolute_output
        .file_name()
        .context("output path has no file name")?;
    let destination = parent.join(name);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&destination)
        .context("redacted output already exists or cannot be created")?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()?;
    Ok(destination)
}

pub async fn analyze_file(input: &Path, config: &Config) -> Result<SemanticReport> {
    let text = read_selected_text(input, config)?;
    analyze_text(&text, config).await
}

async fn analyze_text(text: &str, config: &Config) -> Result<SemanticReport> {
    let chunks = split_chunks(
        text,
        config.semantic_review.chunk_chars,
        config.semantic_review.max_chunks,
    )?;
    let review = async {
        let mut probe_inputs = Vec::new();
        for probe in &config.semantic_review.probes {
            for locale in &config.product.supported_locales {
                let description = probe.descriptions.get(locale).with_context(|| {
                    format!("semantic probe {} lacks locale {locale}", probe.category)
                })?;
                probe_inputs.push((probe.category.as_str(), description.as_str()));
            }
        }
        let mut probe_vectors = Zeroizing::new(Vec::with_capacity(probe_inputs.len()));
        for batch in probe_inputs.chunks(config.semantic_review.batch_size) {
            let texts: Vec<&str> = batch.iter().map(|(_, text)| *text).collect();
            let vectors = Zeroizing::new(embedding::embed_many(&texts, config).await?);
            for ((category, _), vector) in batch.iter().zip(vectors.iter()) {
                probe_vectors.push(((*category).to_owned(), vector.clone()));
            }
        }
        let mut cues = Vec::new();
        for (batch_index, batch) in chunks.chunks(config.semantic_review.batch_size).enumerate() {
            let texts: Vec<&str> = batch.iter().map(|chunk| chunk.as_str()).collect();
            let vectors = Zeroizing::new(embedding::embed_many(&texts, config).await?);
            for (offset, vector) in vectors.iter().enumerate() {
                let segment = batch_index * config.semantic_review.batch_size + offset + 1;
                let mut best_by_category = BTreeMap::<&str, f32>::new();
                for (category, probe_vector) in probe_vectors.iter() {
                    let similarity = cosine(vector, probe_vector)?;
                    best_by_category
                        .entry(category)
                        .and_modify(|score| *score = score.max(similarity))
                        .or_insert(similarity);
                }
                for (category, similarity) in best_by_category {
                    if similarity >= config.semantic_review.similarity_threshold {
                        cues.push(SemanticCue {
                            segment,
                            category: category.to_owned(),
                            similarity,
                        });
                    }
                }
            }
        }
        Ok::<_, anyhow::Error>(SemanticReport {
            segments_analyzed: chunks.len(),
            cues,
        })
    };
    tokio::time::timeout(
        std::time::Duration::from_secs(config.semantic_review.max_review_seconds),
        review,
    )
    .await
    .context("local semantic review exceeded its configured total duration")?
}

fn read_selected_text(input: &Path, config: &Config) -> Result<Zeroizing<String>> {
    let mut input_options = fs::OpenOptions::new();
    input_options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        input_options.custom_flags(libc::O_NOFOLLOW);
    }
    let mut input_file = input_options
        .open(input)
        .context("cannot open the selected document without following symlinks")?;
    let input_meta = input_file.metadata()?;
    if !input_meta.is_file() {
        bail!("selected document must be a regular file and must not be a symlink");
    }
    if input_meta.len() > config.security.max_document_bytes {
        bail!("selected document exceeds the configured size limit");
    }
    if let Some(extension) = input.extension().and_then(|value| value.to_str()) {
        let extension = format!(".{}", extension.to_ascii_lowercase());
        if !config
            .documents
            .allowed_text_extensions
            .contains(&extension)
        {
            bail!(
                "unsupported document extension; only configured plain-text formats are accepted"
            );
        }
    } else {
        bail!("selected document has no recognized file extension");
    }
    let mut bytes = Zeroizing::new(Vec::new());
    Read::by_ref(&mut input_file)
        .take(config.security.max_document_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .context("cannot read selected document locally")?;
    if bytes.len() as u64 > config.security.max_document_bytes {
        bail!("selected document exceeds the configured size limit");
    }
    let text = std::str::from_utf8(&bytes).context("selected file is not valid UTF-8 text")?;
    Ok(Zeroizing::new(text.to_owned()))
}

fn split_chunks(
    text: &str,
    chunk_chars: usize,
    max_chunks: usize,
) -> Result<Vec<Zeroizing<String>>> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_chars = 0usize;
    for character in text.chars() {
        current.push(character);
        current_chars += 1;
        if current_chars >= chunk_chars {
            if chunks.len() >= max_chunks {
                bail!("document exceeds the configured semantic review segment limit");
            }
            chunks.push(Zeroizing::new(std::mem::take(&mut current)));
            current_chars = 0;
        }
    }
    if !current.is_empty() {
        if chunks.len() >= max_chunks {
            bail!("document exceeds the configured semantic review segment limit");
        }
        chunks.push(Zeroizing::new(current));
    }
    if chunks.is_empty() {
        bail!("selected document contains no text to analyze");
    }
    Ok(chunks)
}

fn cosine(left: &[f32], right: &[f32]) -> Result<f32> {
    if left.is_empty() || left.len() != right.len() {
        bail!("embedding vectors have incompatible dimensions");
    }
    let dot: f32 = left.iter().zip(right).map(|(a, b)| a * b).sum();
    let left_norm = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let right_norm = right.iter().map(|value| value * value).sum::<f32>().sqrt();
    if left_norm == 0.0 || right_norm == 0.0 {
        return Ok(0.0);
    }
    let score = dot / (left_norm * right_norm);
    if score.is_finite() {
        Ok(score)
    } else {
        bail!("embedding similarity score is invalid");
    }
}

fn passes_luhn(value: &str) -> bool {
    let digits: Vec<u32> = value.chars().filter_map(|char| char.to_digit(10)).collect();
    if !(13..=19).contains(&digits.len()) {
        return false;
    }
    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(index, digit)| {
            if index % 2 == 1 {
                let doubled = digit * 2;
                if doubled > 9 { doubled - 9 } else { doubled }
            } else {
                *digit
            }
        })
        .sum();
    sum % 10 == 0
}
