use reqwest;
use serde::{Deserialize, Serialize};
use serde_json;
use std::env;
use termion::{input::TermRead, raw::IntoRawMode};
use tokio;

#[derive(Serialize)]
struct OpenAIRequest<'a> {
    prompt: &'a str,
    max_tokens: i32,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    text: String,
}

async fn send_prompt(prompt: &str, api_key: &str) -> Result<String, reqwest::Error> {
    let client = reqwest::Client::new();
    let request_body = OpenAIRequest {
        prompt,
        max_tokens: 150,
    };

    let res = client.post("https://api.openai.com/v1/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await?
        .json::<OpenAIResponse>()
        .await?;

    Ok(res.choices.first().map_or(String::from("No response"), |c| c.text.clone()))
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not found in .env file");

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().into_raw_mode().unwrap();

    writeln!(stdout, "Enter your prompt: ").unwrap();
    stdout.flush().unwrap();

    if let Some(input) = stdin.lock().lines().next() {
        if let Ok(prompt) = input {
            match send_prompt(&prompt, &api_key).await {
                Ok(response) => println!("\nResponse: {}", response),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
    }
}
