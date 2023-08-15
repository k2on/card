use structopt::StructOpt;
use std::fs;
use std::io::{self, Write};
use std::error::Error;
use reqwest::{Client, Url, Response, Method, header};
use serde::{Serialize, Deserialize};

#[derive(StructOpt, Debug)]
#[structopt(name = "card")]
enum CommandCard {
    #[structopt(name = "default")]
    Default,
    /// Authenticate with privacy.com
    Auth,
    /// Create a new card
    Create {
        /// Name/Memo for the card
        name: String,

        /// Amount limit for the card
        amount: u32,
    }
}

#[derive(Debug, Deserialize, Clone)]
struct Cards {
    data: Vec<Card>,
    total_pages: i32,
    page: i32,
}

#[derive(Debug, Deserialize, Clone)]
struct Card {
    created: String,
    token: String,
    last_four: String,
    hostname: String,
    memo: String,
    #[serde(rename = "type")]
    card_type: String,
    spend_limit: u32,
    spend_limit_duration: String,
    state: String,
    funding: Funding,
    auth_rule_tokens: Vec<String>,
    pan: Option<String>,
    cvv: Option<String>,
    exp_month: String,
    exp_year: String,
}

#[derive(Debug, Deserialize, Clone)]
struct Funding {
    created: String,
    token: String,
    #[serde(rename = "type")]
    funding_type: String,
    state: String,
    nickname: Option<String>,
    account_name: String,
    last_four: String,
}

#[derive(Debug, Serialize)]
struct CardCreationPayload {
    #[serde(rename = "type")]
    card_type: String,
    memo: String,
    spend_limit: u32,
    spend_limit_duration: String,
    state: String,
}

const BASE_URL: &str = "https://api.privacy.com/v1/";

struct ApiClient {
    client: Client,
    api_key: String,
}

impl ApiClient {
    fn new(api_key: &str) -> Self {
        ApiClient {
            client: Client::new(),
            api_key: api_key.to_string(),
        }
    }

    async fn request<T: Serialize>(
        &self,
        method: Method,
        endpoint: &str,
        body: Option<T>,
    ) -> Result<Response, Box<dyn Error>> {
        let url = Url::parse(BASE_URL)?.join(endpoint)?;

        // println!("{url}");

        let mut request = self.client.request(method, url);

        // Inject the Authorization header
        request = request.header(header::AUTHORIZATION, format!("api-key {}", self.api_key));

        // If there's a body provided, serialize it to JSON and set to the request
        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await?;
        let text = response.text().await.unwrap();
        println!("{text:?}");

        todo!()
        // Ok(response)
    }

    async fn get(&self, endpoint: &str) -> Result<Response, Box<dyn Error>> {
        self.request(Method::GET, endpoint, None::<()>).await
    }

    async fn post<T: Serialize>(&self, endpoint: &str, body: T) -> Result<Response, Box<dyn Error>> {
        self.request(Method::POST, endpoint, Some(body)).await
    }
    async fn list(&self) -> Result<Cards, Box<dyn Error>> {
        let response = self.get("cards").await?;
        // println!("{}", response.text().await.unwrap());
        let card: Cards = response.json().await?;
        // todo!()
        Ok(card)
    }

    async fn create_card(&self, payload: CardCreationPayload) -> Result<Card, Box<dyn Error>> {
        let response = self.post("cards", payload).await?;
        let card: Card = response.json().await?;
        Ok(card)
    }
}

fn get_xdg_data_home() -> Option<std::path::PathBuf> {
    std::env::var("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|_| {
            std::env::var("HOME").map(|home| {
                let mut path = std::path::PathBuf::from(home);
                path.push(".local/share/card/key");
                path
            })
        })
        .ok()
}

async fn handle_default_command() {
    if let Some(xdg_data_path) = get_xdg_data_home() {
        if xdg_data_path.exists() {
            match fs::read_to_string(&xdg_data_path) {
                Ok(content) => {
                    let client = ApiClient::new(&content);
                    let cards = client.list().await.unwrap();
                    let open: Vec<Card> = cards.data
                        .iter()
                        .filter_map(|card| if card.state == "OPEN" { Some(card.to_owned()) } else { None })
                        .collect();
                    for card in open {
                        println!("{}", card.memo);
                    }

                },
                Err(e) => {
                    eprintln!("Error reading key file: {}", e);
                }
            }
        } else {
            println!("Please run 'card auth' to login to privacy.com");
        }
    } else {
        println!("Cannot determine XDG data directory.");
    }
}

fn handle_auth_command() {
    let auth_url = "https://app.privacy.com/account";

    println!("Opening '{auth_url}' in your browser...");

    if open::that(auth_url).is_err() {
        eprintln!("Failed to open the authentication URL.");
        return;
    }

    print!("Enter your secret key: ");
    io::stdout().flush().unwrap(); // flush the prompt to ensure it appears before the hidden input

    let secret_key = rpassword::read_password().unwrap();


    if let Some(xdg_data_path) = get_xdg_data_home() {
        if let Some(parent_dir) = xdg_data_path.parent() {
            if let Err(e) = fs::create_dir_all(parent_dir) {
                eprintln!("Failed to create directory: {}", e);
                return;
            }
        }

        match fs::write(&xdg_data_path, &secret_key) {
            Ok(_) => println!("Key saved successfully."),
            Err(e) => eprintln!("Failed to save key: {}", e),
        }
    } else {
        println!("Cannot determine XDG data directory.");
    }
}

async fn handle_create_command(name: String, amount: u32) {
    if let Some(xdg_data_path) = get_xdg_data_home() {
        if xdg_data_path.exists() {
            match fs::read_to_string(&xdg_data_path) {
                Ok(content) => {
                    let client = ApiClient::new(&content);
                    client.create_card(CardCreationPayload {
                        card_type: "SINGLE_USE".to_owned(),
                        memo: name,
                        spend_limit: amount,
                        spend_limit_duration: "TRANSACTION".to_owned(),
                        state: "OPEN".to_owned(),
                    }).await.unwrap();
                    println!("Card created");

                },
                Err(e) => {
                    eprintln!("Error reading key file: {}", e);
                }
            }
        } else {
            println!("Please run 'card auth' to login to privacy.com");
        }
    } else {
        println!("Cannot determine XDG data directory.");
    }
}

#[tokio::main]
async fn main() {
    if std::env::args().len() <= 1 {
        handle_default_command().await;
    } else {
        let opt = CommandCard::from_args();

        match opt {
            CommandCard::Default => unreachable!(),
            CommandCard::Auth => {
                handle_auth_command();
            },
            CommandCard::Create { name, amount } => {
                handle_create_command(name, amount).await;
            }
        }
    }
}

