# Localization skill

- Supported language tags are `en`, `it`, `fr`, and `es`.
- Use locale files under `config/locales/` as the source of user-facing application text.
- Every locale must provide the same typed keys. Missing keys are a build or startup error; do not silently fall back to English.
- Keep dates, numbers, pluralization, and user-provided text separate from translated strings.
- Preserve placeholders and their types across all locales. Escape values through the template engine; never concatenate untrusted text into templates.
- Keep CLI help understandable in English until localized help is generated from the same locale catalog. Do not duplicate a product message in Rust source.
- Review translated security, consent, and data-loss wording with fluent reviewers before release; machine translation is not approval.
