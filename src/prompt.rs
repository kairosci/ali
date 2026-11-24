use crate::issue::Issue;
use crate::settings::Config;
use anyhow::Result;
use minijinja::{Environment, context};

pub fn build(issue: &Issue, config: &Config, locale: &str) -> Result<String> {
    let template_text = config.template(&format!("prompts/{locale}.md"))?;
    let mut env = Environment::new();
    env.add_template(locale, &template_text)?;
    let template = env.get_template(locale)?;
    Ok(template.render(context! {
        repository => issue.repository,
        number => issue.number,
        title => issue.title,
        url => issue.web_url,
        state => issue.state,
        author => issue.author,
        labels => issue.labels,
        body => issue.body,
            comments => issue.comments.iter().map(|c| (c.author.as_deref(), c.body.as_deref())).collect::<Vec<_>>(),
    })?)
}
