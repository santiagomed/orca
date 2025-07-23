use handlebars::{Context, Handlebars as Registry, Helper, HelperDef, HelperResult, Output, RenderContext, Renderable};
use serde::{Deserialize, Serialize};

use std::fmt::{self, Display, Formatter};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ChatPrompt(pub(crate) Vec<Message>);

impl From<&str> for Role {
    fn from(role: &str) -> Self {
        match role {
            "system" => Role::System,
            "user" => Role::User,
            "assistant" => Role::Assistant,
            _ => Role::System,
        }
    }
}

impl Display for Role {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
        }
    }
}

impl Display for ChatPrompt {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(&self.0).unwrap_or_default())
    }
}

impl ChatPrompt {
    pub fn to_vec(&self) -> Vec<Message> {
        self.0.clone()
    }

    pub fn to_vec_ref(&self) -> &Vec<Message> {
        &self.0
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Message {
    /// The message role (system, user, assistant)
    pub role: Role,

    /// The message text
    pub content: String,

    pub refusal: Option<String>,
}

impl Message {
    pub fn new(role: Role, content: &str) -> Message {
        Message {
            role,
            content: content.to_string(),
            refusal: None,
        }
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap_or_default())
    }
}

#[derive(Clone)]
pub struct RoleHelper;
#[derive(Clone)]
pub struct ChatHelper;

impl HelperDef for RoleHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _r: &'reg Registry<'reg>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let role = h.name();
        let content = h.template().map_or(Ok(String::new()), |t| t.renders(_r, ctx, rc))?;

        let json = format!(
            r#"{{"role": "{}", "content": "{}"}},"#,
            role,
            clean_string(content.trim())
        );
        out.write(&json)?;
        Ok(())
    }
}

impl HelperDef for ChatHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _r: &'reg Registry<'reg>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let content = h.template().map_or(Ok(String::new()), |t| t.renders(_r, ctx, rc))?;
        let content = remove_last_comma(content.as_str());
        let json = format!(r#"[{}]"#, content);
        out.write(&json)?;
        Ok(())
    }
}

impl Copy for RoleHelper {}
impl Copy for ChatHelper {}

pub fn remove_last_comma(content: &str) -> String {
    content.trim().trim_end_matches(',').to_string()
}

fn clean_string(content: &str) -> String {
    content
        .chars()
        .filter(|&c| c > '\u{1F}')
        .map(|c| match c {
            '"' => "\\\"".to_string(),
            '\\' => "\\\\".to_string(),
            '/' => "\\/".to_string(),
            '\u{08}' => "\\b".to_string(),
            '\u{0C}' => "\\f".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\t' => "\\t".to_string(),
            '{' => "\\u007B".to_string(),
            '}' => "\\u007D".to_string(),
            '[' => "\\u005B".to_string(),
            ']' => "\\u005D".to_string(),
            ',' => "\\u002C".to_string(),
            ':' => "\\u003A".to_string(),
            '&' => "&amp;".to_string(), // If you want to escape HTML entities
            _ => c.to_string(),
        })
        .collect::<String>()
}

#[cfg(test)]
mod test {
    use super::*;
    use handlebars::Handlebars;
    use serde_json::{from_str, json};

    static SYSTEM_HELPER: RoleHelper = RoleHelper;
    static USER_HELPER: RoleHelper = RoleHelper;
    static ASSISTANT_HELPER: RoleHelper = RoleHelper;
    static CHAT_HELPER: ChatHelper = ChatHelper;

    #[test]
    fn test_chat() {
        let mut handlebars = Handlebars::new();
        handlebars.register_helper("system", Box::new(SYSTEM_HELPER));
        handlebars.register_helper("user", Box::new(USER_HELPER));
        handlebars.register_helper("assistant", Box::new(ASSISTANT_HELPER));
        handlebars.register_helper("chat", Box::new(CHAT_HELPER));

        let template = r#"
            {{#chat}}
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
            {{/chat}}
            "#;

        let data = json!({
            "country": "Brazil"
        });

        let rendered = handlebars.render_template(template, &data).unwrap();

        let messages: Vec<Message> = from_str(&rendered).unwrap();
        assert_eq!(messages.len(), 4);
    }
}
