use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use tokio;
use reqwest::Error as ReqwestError;
use termion::input::TermRead;
use std::io::{Write, stdout, stdin};

#[derive(Deserialize, Debug)]
struct ApiResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
struct Choice {
    index: i32,
    message: Message,
    finish_reason: String,
}

#[derive(Deserialize, Debug)]
struct Message {
    role: String, // "system", "user", or "assistant"
    content: String,
}

fn get_prompt() -> String {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let stdin = stdin();
    let mut stdin = stdin.lock();

    stdout.write_all(b"prompt: ").unwrap();
    stdout.flush().unwrap();

    let prompt = stdin.read_line();

    if let Ok(Some(prompt)) = prompt {
        stdout.write_all(prompt.as_bytes()).unwrap();
        stdout.write_all(b"\n").unwrap();
        prompt
    } else {
        stdout.write_all(b"Error\n").unwrap();
        "Answer with nothing but a soft curse word like 'heck' but Italian one".to_string()
    }
}

async fn send_prompt(prompt: &str, api_key: &str) -> Result<String, ReqwestError> {
    let client = reqwest::Client::new();
    let request_body = serde_json::json!({
        "model": "gpt-3.5-turbo",
        "messages": [
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.7
    });

    let res = client.post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await?;

    // Convert the response body to text for debugging and parsing
    let body_text = res.text().await?;

    // Debugging: print the raw response body
    println!("Raw response body: {}", body_text);

    // Attempt to parse the raw text into the ApiResponse struct
    let parsed_response: ApiResponse = serde_json::from_str(&body_text).unwrap();

    // Debug: Print the parsed response
    println!("Parsed response: {:?}", parsed_response);

    // Extract the last message content from the first choice (assuming there's at least one)
    let response_message = parsed_response.choices.get(0)
        .map_or_else(|| "No response found.".to_string(), |choice| choice.message.content.clone());

    Ok(response_message)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let api_key = env::var("OPEN_AI_API_KEY").expect("OPEN_AI_API_KEY not found in .env file");

    // Set your predefined prompt here
    let prompt = get_prompt();

    match send_prompt(&prompt, &api_key).await {
        Ok(response) => println!("\nResponse: {}", response),
        Err(e) => eprintln!("Error: {}", e),
    }

    Ok(())
}
