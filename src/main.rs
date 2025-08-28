use std::env;
use dotenvy::dotenv;

use groq_api_rust::{
    GroqClient, ChatCompletionMessage, ChatCompletionRoles, ChatCompletionRequest,
};

fn main() {
    dotenv().ok();

    let api_key = env::var("GROQ_API_KEY")
        .expect("GROQ_API_KEY must be set in .env file");

    let client = GroqClient::new(api_key, None);

    let messages = vec![ChatCompletionMessage {
        role: ChatCompletionRoles::User,
        content: "Hello Groq, say something nice!".to_string(),
        name: None,
    }];

    let request = ChatCompletionRequest::new("llama3-70b-8192", messages);

    let response = client.chat_completion(request).expect("API request failed");

    if let Some(choice) = response.choices.first() {
        println!("Groq says: {}", choice.message.content);
    } else {
        println!("No response received.");
    }
}
