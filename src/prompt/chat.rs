use async_openai::types::Role as R;
use handlebars::{Context, Handlebars as Registry, Helper, HelperDef, HelperResult, Output, RenderContext, Renderable};
use serde::{Deserialize, Serialize};

use std::fmt::{self, Display, Formatter};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Role(pub R);

impl From<&str> for Role {
    fn from(role: &str) -> Self {
        match role {
            "system" => Role(R::System),
            "user" => Role(R::User),
            "assistant" => Role(R::Assistant),
            "function" => Role(R::Function),
            _ => Role(R::System),
        }
    }
}

impl Display for Role {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.0 {
            R::System => write!(f, "system"),
            R::User => write!(f, "user"),
            R::Assistant => write!(f, "assistant"),
            R::Function => write!(f, "function"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    /// The message role (system, user, assistant)
    pub role: Role,

    /// The message text
    pub content: String,
}

impl Message {
    pub fn new(role: Role, content: &str) -> Message {
        Message {
            role,
            content: content.to_string(),
        }
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "[{}] {}", self.role, self.content)
    }
}

#[derive(Clone)]
pub struct RoleHelper;

impl HelperDef for RoleHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        _r: &'reg Registry<'reg>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let role = h.name();
        let content = h.template().map_or(Ok(String::new()), |t| t.renders(_r, ctx, rc))?;
        let json = format!(r#"{{"role": "{}", "content": "{}"}},"#, role, content.trim());
        out.write(&json)?;
        Ok(())
    }
}

pub static SYSTEM_HELPER: RoleHelper = RoleHelper;
pub static USER_HELPER: RoleHelper = RoleHelper;
pub static ASSISTANT_HELPER: RoleHelper = RoleHelper;

#[cfg(test)]
mod test {
    use super::*;
    use handlebars::Handlebars;
    use serde_json::{from_str, json};

    #[test]
    fn test_chat() {
        let mut handlebars = Handlebars::new();
        handlebars.register_helper("system", Box::new(SYSTEM_HELPER.clone()));
        handlebars.register_helper("user", Box::new(USER_HELPER.clone()));
        handlebars.register_helper("assistant", Box::new(ASSISTANT_HELPER.clone()));

        let template = r#"
            {{#system}}
            You are an expert in world capitals.
            {{/system}}
            {{#user}}
            What is the capital of France?
            {{/user}}
            {{#assistant}}
            The capital of France is Paris.
            {{/assistant}}
            {{#user}}
            What is the capital of {{country}}?
            {{/user}}
            "#;

        let data = json!({
            "country": "Brazil"
        });

        let rendered = handlebars.render_template(template, &data).unwrap();
        let json_rendered = format!("[{}]", rendered.trim().trim_end_matches(','));
        println!("{}", json_rendered);

        let messages: Vec<Message> = from_str(&json_rendered).unwrap();
        assert_eq!(messages.len(), 4);
    }
}
