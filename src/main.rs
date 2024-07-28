use rand::Rng;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::env;
use std::fmt;
use std::io::{self, BufRead, BufReader, Write};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use termion::{color, style, terminal_size};

#[derive(Deserialize, Debug)]
struct ApiResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
struct Choice {
    message: Message,
}

#[derive(Deserialize, Debug, Clone)]
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

fn get_prompt() -> (String, String) {
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
    let line: String = (0..width * 2)
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
    println!();

    // Prompt for input
    //writeln!(stdout, "prompt: (type 'end' on a new line to finish)").unwrap();
    stdout.flush().unwrap();

    let mut prompt = String::new();
    let mut model = "gpt-4o-mini"; // Default model
    for line in stdin.lines() {
        let line = line.expect("Failed to read line");
        if line == "//3" {
            model = "gpt-3.5-turbo";
            break;
        } else if line == "//4" {
            model = "gpt-4o";
            break;
        } else if line == "///" {
            break;
        }
        prompt.push_str(&line);
        prompt.push('\n');
    }

    (prompt, model.to_string())
}

fn send_prompt(
    prompt: &str,
    api_key: &str,
    model: &str,
    history: &Arc<Mutex<Vec<Message>>>,
) -> Result<String, AppError> {
    println!();
    let mut history = history.lock().unwrap();
    history.push(Message {
        role: "user".to_string(),
        content: prompt.to_string(),
    });

    let mut total_chars: usize = history.iter().map(|msg| msg.content.len()).sum();
    while total_chars > 20000 && !history.is_empty() {
        let removed = history.remove(0);
        total_chars -= removed.content.len();
    }

    let messages: Vec<_> = history
        .iter()
        .map(|msg| serde_json::json!({"role": msg.role, "content": msg.content}))
        .collect();

    let client = Client::new();
    let request_body = serde_json::json!({
        "model": model,
        "messages": messages,
        "temperature": 0.4
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

    let history = Arc::new(Mutex::new(Vec::new()));

    loop {
        let (prompt, model) = get_prompt();
        if prompt.trim().is_empty() || prompt.trim() == "exit" {
            break;
        }

        let (tx, rx) = mpsc::channel();

        let prompt_clone = prompt.clone();
        let api_key_clone = api_key.clone();
        let history_clone = Arc::clone(&history);
        // Spawn a new thread for the blocking send_prompt operation
        thread::spawn(move || {
            let result = send_prompt(
                &prompt_clone,
                &api_key_clone as &str,
                &model as &str,
                &history_clone,
            );
            tx.send(result).expect("Failed to send result over channel");
        });

        let loader_chars = "☺☻♥♦♣♠•○♂♀";
        let loaderchars_len = loader_chars.chars().count();
        let mut rng = rand::thread_rng();
        loop {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(result) => {
                    // Process the result
                    match result {
                        Ok(response) => println!("\n\n{}", response),
                        Err(e) => eprintln!("Error on send_prompt: {}", e),
                    }
                    break; // Break the loop when the result is received
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Update and display the animation frame
                    let idx = rng.gen_range(0..loaderchars_len);
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightCyan),
                        loader_chars.chars().nth(idx).unwrap(),
                        color::Fg(color::Reset)
                    );
                    io::stdout().flush().unwrap();
                }
                Err(_) => {
                    eprintln!("\nThe thread handling the request has terminated unexpectedly.");
                    break;
                }
            }
        }
        println!();
    }

    Ok(())
}
