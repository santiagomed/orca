// use async_openai::types::{ChatCompletionRequestMessage, ChatCompletionRequestMessageArgs};

// use crate::prompt::chat::Message;

// impl From<Message> for ChatCompletionRequestMessage {
//     /// Convert a Message into a ChatCompletionRequestMessage
//     fn from(message: Message) -> Self {
//         ChatCompletionRequestMessageArgs::default()
//             .role::<async_openai::types::Role>(message.role.0)
//             .content(message.content)
//             .build()
//             .unwrap()
//     }
// }

// /// A vector of ChatCompletionRequestMessage
// pub struct RequestMessages(Vec<ChatCompletionRequestMessage>);

// /// Trait for converting a vector of Messages into a vector of ChatCompletionRequestMessage
// impl From<Vec<Message>> for RequestMessages {
//     fn from(messages: Vec<Message>) -> Self {
//         let mut request_messages = Vec::new();
//         for message in messages {
//             request_messages.push(message.into());
//         }
//         RequestMessages(request_messages)
//     }
// }

// /// Trait for converting a RequestMessages into a vector of ChatCompletionRequestMessage
// impl From<RequestMessages> for Vec<ChatCompletionRequestMessage> {
//     fn from(messages: RequestMessages) -> Self {
//         messages.0
//     }
// }
