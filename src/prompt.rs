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
