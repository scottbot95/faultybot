use std::io::{stdin, stdout, Write};
use dotenvy::dotenv;
use openai::{
    chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole},
    models::ModelID
};

#[tokio::main]
async fn main() {
    dotenv().expect(".env file not found");

    let mut messages = vec![ChatCompletionMessage {
        role: ChatCompletionMessageRole::System,
        content: "You are FaultyGPT, a large language model built into a command line interface. You are helpful, but your responses are always snarky and a bit rude.".to_string(),
        name: None,
    }];

    loop {
        print!("User: ");
        stdout().flush().unwrap();

        let mut user_message_content = String::new();
        // let user_message_content = "Hello, what is your name?".to_string();

        stdin().read_line(&mut user_message_content).unwrap();
        messages.push(ChatCompletionMessage {
            role: ChatCompletionMessageRole::User,
            content: user_message_content,
            name: None,
        });

        let chat_completion = ChatCompletion::builder(ModelID::Gpt3_5Turbo, messages.clone())
            .create()
            .await
            .unwrap()
            .unwrap();
        let returned_message = chat_completion.choices.first().unwrap().message.clone();

        println!(
            "{:#?}: {}",
            &returned_message.role,
            &returned_message.content.trim()
        );

        messages.push(returned_message);
    }
}
