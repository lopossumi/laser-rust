use std::fs::OpenOptions;
use std::thread;
use std::time::Duration;
use chrono::{Local, Days};
use std::io::{prelude::*, SeekFrom};

mod timeslot;
use timeslot::Timeslot;

use once_cell::sync::Lazy;
struct Config {
    telegram_bot_token: String,
    telegram_chat_id: String,
    polling_interval: u64,
    days: u64
}
static CONFIG: Lazy<Config> = Lazy::new(|| Config {
    telegram_bot_token: std::env::var("TELEGRAM_BOT_TOKEN").unwrap(),
    telegram_chat_id: std::env::var("TELEGRAM_CHAT_ID").unwrap(),
    polling_interval: 600,
    days: 14
});

fn main() {
    // Fetch data from API every 10 minutes
    loop {
        fetch_data();
        thread::sleep(Duration::from_secs(CONFIG.polling_interval));
    }
}

fn fetch_data() {
    println!("Fetching data...");
    let api_data = fetch_api_data();

    let opening_times = parse_opening_times(&api_data);
    let reservations = parse_reservations(&api_data);
    let available_times = timeslot::get_available_times(&opening_times, &reservations);

    // Read existing data from a txt file called available_times.csv. If the file does not exist, create a new empty file.
    let mut file = match OpenOptions::new()
        .read(true)
        .write(true)
        .open("available_times.csv") {
        Ok(file) => file,
        Err(_) => panic!("Failed to open file")
    };

    let existing_available_times = read_existing_available_times(&mut file);
    let new_times = &available_times
        .iter()
        .filter(|time| !existing_available_times.contains(time))
        .collect::<Vec<&Timeslot>>();

    // Write available times to file, replacing the existing file contents.
    update_file(&mut file, &available_times);

    // Send telegram message with new times.
    send_telegram_message(new_times);
}

fn read_existing_available_times(file: &mut std::fs::File) -> Vec<Timeslot> {
    // Read existing available times from file.
    let mut existing_available_times: Vec<Timeslot> = Vec::new();
    // If the file is empty, do nothing.
    if file.metadata().unwrap().len() != 0 {
        // read file contents to string array
        let mut file_contents = String::new();
        file.read_to_string(&mut file_contents).expect("Failed to read file");
        // split string array by newlines
        let file_lines = file_contents.split("\n");
        // parse each line as a Timeslot and add it to existing_available_times
        // The lines are in the following format:
        // 2021-09-01T10:00:00+03:00,2021-09-01T11:00:00+03:00
        for line in file_lines {
            if line == "" {
                continue;
            }
            let timeslot = Timeslot {
                start: line.split(",").collect::<Vec<&str>>()[0].to_owned(),
                end: line.split(",").collect::<Vec<&str>>()[1].to_owned(),
            };
            existing_available_times.push(timeslot);
        }
    }
    return existing_available_times;
}


/// Make an API request to api.hel.fi/respa and return response as a JSON object.
///
/// # Panics
///
/// Panics if the API request fails or if the JSON parsing fails.
fn fetch_api_data() -> serde_json::Value {
    let current_time = Local::now();
    let start_date = current_time.format("%Y-%m-%d").to_string();
    let end_date = (current_time.checked_add_days(Days::new(CONFIG.days))).unwrap().format("%Y-%m-%d").to_string();

    // Append "T23:59:59" to end_date to get all reservations for the day.
    let end_date = format!("{}T23:59:59", end_date);

    let request_url = format!("https://api.hel.fi/respa/v1/resource/axwzr3i57yba/?start={}&end={}&format=json", start_date, end_date);

    println!("Request URL: {}", request_url);

    let api_response = reqwest::blocking::get(&request_url).expect("Failed to fetch API data").text().unwrap();
    let api_data: serde_json::Value = serde_json::from_str(&api_response).expect("Failed to parse JSON");
    api_data
}


/// Parse all opening times from API data. Return a vector of Timeslot structs.
/// Returns an empty vector on error.
fn parse_opening_times(api_data: &serde_json::Value) -> Vec<Timeslot> {   
    // Get opening hours from API data. If opening hours is null, return an empty vector.
    let binding = Vec::new();
    let opening_hours = api_data["opening_hours"].as_array().unwrap_or(&binding);
        
    // Create a Vec<Timeslot> from opening hours
    let mut opening_times: Vec<Timeslot> = Vec::new();
    for opening_hour in opening_hours {
        // Skip opening hours that are null
        if opening_hour["opens"].is_null() || opening_hour["closes"].is_null() {
            continue;
        }

        // Create Timeslot from opening hour
        let timeslot = Timeslot { 
            start: opening_hour["opens"].as_str().unwrap().to_owned(), 
            end: opening_hour["closes"].as_str().unwrap().to_owned() 
        };

        // Add Timeslot to opening times
        opening_times.push(timeslot);
    }

    return opening_times;
}

/// Parse all reservations times from API data. Return a vector of Timeslot structs.
/// Returns an empty vector on error.
fn parse_reservations(api_data: &serde_json::Value) -> Vec<Timeslot> {
    // Get reservations from API data
    let binding = Vec::new();
    let reservations = api_data["reservations"].as_array().unwrap_or(&binding);

    // Create a Vec<Timeslot> from reservations
    let mut reservation_times: Vec<Timeslot> = Vec::new();
    for reservation in reservations {
        // Skip reservations that are null
        if reservation["begin"].is_null() || reservation["end"].is_null() {
            continue;
        }

        // Create Timeslot from reservation
        let timeslot = Timeslot { 
            start: reservation["begin"].as_str().unwrap().to_owned(), 
            end: reservation["end"].as_str().unwrap().to_owned() 
        };

        // Add Timeslot to reservation times
        reservation_times.push(timeslot);
    }

    return reservation_times;
}

fn update_file(file: &mut std::fs::File, available_times: &[Timeslot]) {
    file.set_len(0).expect("Failed to truncate file");
    file.seek(SeekFrom::Start(0)).unwrap();
    for time in available_times {
        file.write_all(format!("{},{}\n", time.start, time.end).as_bytes()).expect("Failed to write to file");
    }
}

fn send_telegram_message(new_times: &Vec<&Timeslot>) {
    // Send telegram message with new available times
    // If there are no new available times, do nothing.
    if new_times.len() == 0 {
        println!("No new available times");
        return;
    }

    // Create message
    let mut message = String::new();
    message.push_str("New available times:%0A");
    for time in new_times {
        message.push_str(&format!("{}%0A", time));
    }

    let bot_token = CONFIG.telegram_bot_token.as_str();
    let chat_id = CONFIG.telegram_chat_id.as_str();

    let url = format!("https://api.telegram.org/bot{}/sendMessage?chat_id={}&text={}", 
        bot_token, 
        chat_id, 
        message);

    let response = reqwest::blocking::get(&url).expect("Failed to send message");
    println!("Telegram response: {}", response.text().unwrap());
}
