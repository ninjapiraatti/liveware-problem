use serde::{Deserialize, Serialize};
use reqwest::blocking::Client;
use std::io::{self, Write, BufRead, BufReader};
use termion::input::TermRead;
use std::env;

#[derive(Deserialize, Debug)]
struct ApiResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
struct Choice {
    message: Message,
}

#[derive(Deserialize, Debug)]
struct Message {
    role: String,
    content: String,
}

fn get_prompt() -> String {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let stdin = io::stdin();
    let stdin = BufReader::new(stdin);

    writeln!(stdout, "prompt: (type 'end' on a new line to finish)").unwrap();
    stdout.flush().unwrap();

    let mut prompt = String::new();
    for line in stdin.lines() {
        let line = line.expect("Failed to read line");
        if line == "end" {
            break;
        }
        prompt.push_str(&line);
        prompt.push('\n');
    }

    prompt
}

fn send_prompt(prompt: &str, api_key: &str) -> Result<String, reqwest::Error> {
    let client = Client::new();
    let request_body = serde_json::json!({
        "model": "gpt-3.5-turbo",
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0.7
    });

    let res = client.post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()?;

    let body_text = res.text()?;
    println!("Raw response body: {}", body_text); // For debugging

    let parsed_response: ApiResponse = serde_json::from_str(&body_text).unwrap();

    let response_message = parsed_response.choices.get(0)
        .map_or_else(|| "No response found.".to_string(), |choice| choice.message.content.clone());

    Ok(response_message)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let api_key = env::var("OPEN_AI_API_KEY").expect("OPEN_AI_API_KEY not found in .env file");
    loop {
        let prompt = get_prompt();
        if prompt.trim().is_empty() || prompt.trim() == "exit" {
            break;
        }

        match send_prompt(&prompt, &api_key as &str) {
            Ok(response) => println!("\nResponse: {}", response),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    Ok(())
}
