#![allow(dead_code)]

#[path = "../src/adapters.rs"]
mod adapters;
#[path = "../src/issue.rs"]
mod issue;
#[path = "../src/prompt.rs"]
mod prompt;
#[path = "../src/settings.rs"]
mod settings;

use issue::{Comment, Issue};
use settings::Config;
use std::path::Path;

#[test]
fn prompt_profiles_render_for_every_agent_and_locale() {
    let config = Config::load(Path::new("config")).expect("repository configuration parses");
    let issue = Issue {
        repository: "owner/repository".to_owned(),
        number: 42,
        title: "Synthetic rendering fixture".to_owned(),
        state: "open".to_owned(),
        author: "fixture-user".to_owned(),
        body: Some("Synthetic issue content only.".to_owned()),
        labels: vec!["verification".to_owned()],
        comments: vec![Comment {
            author: Some("reviewer".to_owned()),
            body: Some("Synthetic comment only.".to_owned()),
        }],
        web_url: "https://example.invalid/owner/repository/issues/42".to_owned(),
    };

    for locale in &config.product.supported_locales {
        let generic =
            prompt::build_for_agent(&issue, &config, locale, None).expect("generic prompt renders");
        assert!(generic.contains("Synthetic rendering fixture"));
        for agent in &config.agents {
            let rendered = prompt::build_for_agent(&issue, &config, locale, Some(&agent.id))
                .expect("agent-specific prompt renders");
            assert!(rendered.contains(&agent.labels[locale]));
            assert!(rendered.contains("Synthetic issue content only."));
            assert!(!rendered.contains("{{"));
        }
    }
}
