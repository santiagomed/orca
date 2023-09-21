pub mod context;
pub mod error;
pub mod prompt;

use serde;
use serde::Serialize;
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Serialize, PartialEq, Debug, Clone)]
pub struct Message {
    /// The message role (system, user, ai)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,

    /// The message text
    pub message: String,
}

impl Message {
    pub fn single(message: &str) -> Message {
        Message {
            role: None,
            message: message.to_string(),
        }
    }

    pub fn chat(role: Role, message: &str) -> Message {
        Message {
            role: Some(role),
            message: message.to_string(),
        }
    }

    pub fn into_vec(v: Vec<(&str, &str)>) -> Vec<Message> {
        let mut messages = Vec::new();
        for (role, message) in v {
            messages.push(Message::chat(role.into(), message));
        }
        messages
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match &self.role {
            Some(role) => write!(f, "[{}] {}", role, self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

#[derive(Debug, Serialize, Clone, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    #[default]
    User,
    Ai,
    Function,
}

/// Trait for converting a string to a role
impl From<&str> for Role {
    fn from(s: &str) -> Role {
        match s {
            "system" => Role::System,
            "user" => Role::User,
            "ai" => Role::Ai,
            _ => panic!("Invalid role: {}", s),
        }
    }
}

impl Display for Role {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Ai => write!(f, "ai"),
            Role::Function => write!(f, "function"),
        }
    }
}
