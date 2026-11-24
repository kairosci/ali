use crate::issue::Issue;
use crate::settings::Config;
use anyhow::{Context, Result, bail};
use minijinja::{Environment, context};

pub fn build_for_agent(
    issue: &Issue,
    config: &Config,
    locale: &str,
    agent_id: Option<&str>,
) -> Result<String> {
    let template_text = config.template(&format!("prompts/{locale}.md"))?;
    let mut env = Environment::new();
    env.add_template(locale, &template_text)?;
    let template = env.get_template(locale)?;
    let mut rendered = template.render(context! {
        repository => issue.repository,
        number => issue.number,
        title => issue.title,
        url => issue.web_url,
        state => issue.state,
        author => issue.author,
        labels => issue.labels,
        body => issue.body,
            comments => issue.comments.iter().map(|c| (c.author.as_deref(), c.body.as_deref())).collect::<Vec<_>>(),
    })?;
    if let Some(agent_id) = agent_id {
        let agent = config
            .agents
            .iter()
            .find(|agent| agent.id == agent_id)
            .with_context(|| format!("unknown configured agent {agent_id}"))?;
        let label = agent
            .labels
            .get(locale)
            .context("validated agent label is missing")?;
        let path = format!("agent-prompts/{locale}/{}.md", agent.prompt_profile);
        let source = config.template(&path)?;
        let template_name = format!("agent-{agent_id}-{locale}");
        env.add_template(&template_name, &source)?;
        let agent_prompt = env
            .get_template(&template_name)?
            .render(context! { agent => label })?;
        if agent_prompt.trim().is_empty() {
            bail!("agent-specific prompt template is empty");
        }
        rendered.push_str("\n\n");
        rendered.push_str(&agent_prompt);
    }
    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::build_for_agent;
    use crate::{
        issue::{Comment, Issue},
        settings::Config,
    };
    use std::path::Path;

    #[test]
    fn configured_agent_prompts_render_for_every_locale() {
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
            for agent in &config.agents {
                let prompt = build_for_agent(&issue, &config, locale, Some(&agent.id))
                    .expect("configured localized prompt renders");
                assert!(prompt.contains(&agent.labels[locale]));
                assert!(prompt.contains("Synthetic issue content only."));
                assert!(!prompt.contains("{{"));
            }
        }
    }

    #[test]
    fn generic_prompt_does_not_require_an_agent_profile() {
        let config = Config::load(Path::new("config")).expect("repository configuration parses");
        let issue = Issue {
            repository: "owner/repository".to_owned(),
            number: 42,
            title: "Synthetic rendering fixture".to_owned(),
            state: "open".to_owned(),
            author: "fixture-user".to_owned(),
            body: None,
            labels: vec![],
            comments: vec![],
            web_url: "https://example.invalid/owner/repository/issues/42".to_owned(),
        };
        let prompt = build_for_agent(&issue, &config, "en", None)
            .expect("generic prompt renders without an agent selection");
        assert!(prompt.contains("Synthetic rendering fixture"));
        assert!(!prompt.contains("Target-specific working agreement"));
    }
}
