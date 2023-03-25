use std::error::Error;
use openai::chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole};

pub struct Chat {
    model: String,
    messages: Vec<ChatCompletionMessage>
}

impl Chat {
    pub fn new(model: &str, name: &str) -> Self {
        let system_prompt = format!(r#"
            You are {}, a large language model built into a Discord bot.
            You are helpful, but your responses are always sassy and sometimes rude.
        "#, name);
        Self {
            model: model.to_owned(),
            messages: vec![ChatCompletionMessage {
                role: ChatCompletionMessageRole::System,
                content: system_prompt,
                name: None
            }]
        }
    }

    pub async fn chat_completion(&mut self, message: String, user: Option<String>) -> Result<ChatCompletionMessage, Box<dyn Error>> {
        self.messages.push(ChatCompletionMessage {
            role: ChatCompletionMessageRole::User,
            content: message,
            name: user,
        });

        let completion = ChatCompletion::builder(&self.model, self.messages.clone())
            .create()
            .await??;

        let choice = completion.choices.first().unwrap().message.clone();

        self.messages.push(choice.clone());

        Ok(choice)
    }
}