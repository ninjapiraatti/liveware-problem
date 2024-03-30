use rand::Rng;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::env;
use std::fmt;
use std::io::{self, BufRead, BufReader, Write};
use termion::{color, style, terminal_size};

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

#[derive(Debug)]
enum AppError {
    Reqwest(reqwest::Error),
    NoChoiceFound,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::Reqwest(e) => write!(f, "Request error: {}", e),
            AppError::NoChoiceFound => write!(f, "No choice found in the response"),
        }
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> AppError {
        AppError::Reqwest(err)
    }
}

fn get_prompt() -> String {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let stdin = io::stdin();
    let stdin = BufReader::new(stdin);

    // Predefined string of characters to choose from
    let chars = "░▒▓█⌐¬½¼┴┬─═╧╨╤±≥≤↔-";
    let chars_len = chars.chars().count();

    // Get the terminal width
    let (width, _) = terminal_size().unwrap_or((80, 24)); // Default to 80x24 if unable to get size

    // Random number generator
    let mut rng = rand::thread_rng();

    // Generate two lines of random characters from the predefined string
    let line: String = (0..width)
        .map(|_| {
            let idx = rng.gen_range(0..chars_len);
            chars.chars().nth(idx).unwrap()
        })
        .collect();

    // Drawing the divider with color
    writeln!(
        stdout,
        "{}{}{}{}{}",
        color::Fg(termion::color::Rgb(255, 38, 106),), // Set the color
        style::Bold,                                   // Optional: make it bold
        line,                                          // First line of random characters
        style::Reset,                                  // Reset style
        color::Fg(color::Reset)                        // Reset color
    )
    .unwrap();

    // Repeat for the second line
    let line: String = (0..width)
        .map(|_| {
            let idx = rng.gen_range(0..chars_len);
            chars.chars().nth(idx).unwrap()
        })
        .collect();
    writeln!(
        stdout,
        "{}{}{}{}{}",
        color::Fg(termion::color::Rgb(255, 38, 106),), // Set the color
        style::Bold,                                   // Optional: make it bold
        line,                                          // Second line of random characters
        style::Reset,                                  // Reset style
        color::Fg(color::Reset)                        // Reset color
    )
    .unwrap();

    // Prompt for input
    //writeln!(stdout, "prompt: (type 'end' on a new line to finish)").unwrap();
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

fn send_prompt(prompt: &str, api_key: &str, history: &mut Vec<Message>) -> Result<String, AppError> {
    history.push(Message {
        role: "user".to_string(),
        content: prompt.to_string(),
    });

    let mut total_chars:usize = history.iter().map(|msg| msg.content.len()).sum();
    while total_chars > 10000 && !history.is_empty() {
        let removed = history.remove(0);
        total_chars -= removed.content.len();
    }
    
    let messages: Vec<_> = history.iter().map(|msg| {
        serde_json::json!({"role": msg.role, "content": msg.content})
    }).collect();

    let client = Client::new();
    let request_body = serde_json::json!({
        "model": "gpt-3.5-turbo",
        "messages": messages,
        "temperature": 0.7
    });

    let res = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()?;

    let body_text = res.text()?;
    let parsed_response: ApiResponse = serde_json::from_str(&body_text).unwrap();

    if let Some(choice) = parsed_response.choices.get(0) {
        history.push(Message {
            role: "assistant".to_string(),
            content: choice.message.content.clone(),
        });
        Ok(choice.message.content.clone())
    } else {
        Err(AppError::NoChoiceFound)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let api_key = env::var("OPEN_AI_API_KEY").expect("OPEN_AI_API_KEY not found in .env file");
    
    let mut history = Vec::new();

    loop {
        let prompt = get_prompt();
        if prompt.trim().is_empty() || prompt.trim() == "exit" {
            break;
        }

        match send_prompt(&prompt, &api_key as &str, &mut history) {
            Ok(response) => println!("\nResponse: {}", response),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    Ok(())
}

