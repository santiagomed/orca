use async_openai::types::{ChatCompletionRequestMessage, ChatCompletionRequestMessageArgs};

use crate::prompt::prompt::{Message, Role};

impl From<Role> for async_openai::types::Role {
    fn from(role: Role) -> Self {
        match role {
            Role::System => async_openai::types::Role::System,
            Role::User => async_openai::types::Role::User,
            Role::Ai => async_openai::types::Role::Assistant,
            Role::Function => async_openai::types::Role::Function,
        }
    }
}

impl From<Message> for ChatCompletionRequestMessage {
    fn from(message: Message) -> Self {
        ChatCompletionRequestMessageArgs::default()
            .role::<async_openai::types::Role>(message.role.unwrap_or_default().into())
            .content(message.message)
            .build()
            .unwrap()
    }
}

pub struct RequestMessages(Vec<ChatCompletionRequestMessage>);

impl From<Vec<Message>> for RequestMessages {
    fn from(messages: Vec<Message>) -> Self {
        let mut request_messages = Vec::new();
        for message in messages {
            request_messages.push(message.into());
        }
        RequestMessages(request_messages)
    }
}

impl Into<Vec<ChatCompletionRequestMessage>> for RequestMessages {
    fn into(self) -> Vec<ChatCompletionRequestMessage> {
        self.0
    }
}
