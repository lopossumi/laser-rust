use std::fs::OpenOptions;
use std::string;
use std::thread;
use std::time::Duration;
use chrono::{DateTime, Local};
use std::io::{prelude::*, SeekFrom};

fn main() {
    // Check if TELEGRAM_BOT_TOKEN environment variable is set
    if std::env::var("TELEGRAM_BOT_TOKEN").is_err() {
        println!("TELEGRAM_BOT_TOKEN environment variable is not set");
        return;
    }

    // Check if TELEGRAM_CHAT_ID environment variable is set
    if std::env::var("TELEGRAM_CHAT_ID").is_err() {
        println!("TELEGRAM_CHAT_ID environment variable is not set");
        return;
    }

    // Fetch data from API every 10 minutes
    loop {
        fetch_data();
        thread::sleep(Duration::from_secs(600));
    }
}

fn fetch_data() {
    println!("Fetching data...");
    let api_data = fetch_api_data();

    let opening_times = parse_opening_times(&api_data);
    let reservations = parse_reservations(&api_data);
    let available_times = get_available_times(&opening_times, &reservations);

    println!("Available times:");
    for time in &available_times {
        println!("{}", time);
    }

    // Read existing data from a txt file called available_times.csv. If the file does not exist, create a new empty file.
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("available_times.csv")
        .unwrap();

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

    // Compare existing available times with new available times.
    // If there are new available times, send a telegram message.
    let mut new_times: Vec<Timeslot> = Vec::new();
    for time in &available_times {
        let mut is_new = true;
        for existing_time in &existing_available_times {
            if time.start == existing_time.start && time.end == existing_time.end {
                is_new = false;
                break;
            }
        }
        if is_new {
            new_times.push(time.clone());
        }
    }

    // Write available times to file.
    // Replace existing file contents.
    file.set_len(0).expect("Failed to truncate file");
    file.seek(SeekFrom::Start(0)).unwrap();
    for time in &available_times {
        file.write_all(format!("{},{}\n", time.start, time.end).as_bytes()).expect("Failed to write to file");
    }

    // Send telegram message with new times
    send_telegram_message(&new_times);
}


/// Make an API request to api.hel.fi/respa and return response as a JSON object.
///
/// # Panics
///
/// Panics if the API request fails or if the JSON parsing fails.
fn fetch_api_data() -> serde_json::Value {
    let current_time = Local::now();
    let end_date = current_time + chrono::Duration::days(30);
    let request_url = format!("https://api.hel.fi/respa/v1/resource/axwzr3i57yba/?start={}&end={}&format=json", current_time, end_date);

    let api_response = reqwest::blocking::get(&request_url).expect("Failed to fetch API data").text().unwrap();
    let api_data: serde_json::Value = serde_json::from_str(&api_response).expect("Failed to parse JSON");
    api_data
}


/// Parse all opening times from API data. Return a vector of Timeslot structs.
///
/// # Panics
///
/// Panics if the JSON parsing fails.
fn parse_opening_times(api_data: &serde_json::Value) -> Vec<Timeslot> {   
    // Get opening hours from API data
    let opening_hours = api_data["opening_hours"].as_array().unwrap();
        
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
///
/// # Panics
///
/// Panics if the JSON parsing fails.
fn parse_reservations(api_data: &serde_json::Value) -> Vec<Timeslot> {
    // Get reservations from API data
    let reservations = api_data["reservations"].as_array().unwrap();

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


fn get_available_times(opening_times: &Vec<Timeslot>, reservations: &Vec<Timeslot>) -> Vec<Timeslot> {
    // Iterate over each hour in opening times.
    // If the hour is not in reservations, add it to available times.
    let mut available_times: Vec<Timeslot> = Vec::new();
    for opening_time in opening_times {
        // Get start and end time of opening time
        let start_time = opening_time.start_time();
        let end_time = opening_time.end_time();

        // Iterate over each hour in opening time
        let mut current_time = start_time;
        while current_time < end_time {
            // Check if current time is in reservations
            let mut is_reserved = false;
            for reservation in reservations {
                if current_time >= reservation.start_time() && current_time < reservation.end_time() {
                    is_reserved = true;
                    break;
                }
            }

            // If current time is not in reservations, add it to available times
            if !is_reserved {
                let timeslot = Timeslot { 
                    start: current_time.to_rfc3339(),
                    end: (current_time + chrono::Duration::hours(1)).to_rfc3339(),
                };
                available_times.push(timeslot);
            }

            // Increment current time by 1 hour
            current_time = current_time + chrono::Duration::hours(1);
        }
    }

    // Combine 1 hour timeslots into longer timeslots.
    let mut combined_timeslots: Vec<Timeslot> = Vec::new();
    let mut current_timeslot: Option<Timeslot> = None;

    for timeslot in available_times {
        if let Some(current) = current_timeslot {
            if current.end_time() == timeslot.start_time() {
                // Extend the current timeslot
                current_timeslot = Some(Timeslot {
                    start: current.start,
                    end: timeslot.end,
                });
            } else {
                // Add the current timeslot to the combined timeslots
                combined_timeslots.push(current);
                current_timeslot = Some(timeslot);
            }
        } else {
            current_timeslot = Some(timeslot);
        }
    }

    // Add the last timeslot to the combined timeslots
    if let Some(current) = current_timeslot {
        combined_timeslots.push(current);
    }

    return combined_timeslots;
}

fn send_telegram_message(new_times: &Vec<Timeslot>) {
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

    // Get chat id from environment variable
    let chat_id = std::env::var("TELEGRAM_CHAT_ID").expect("Failed to get chat id from environment variable");

    // Get bot token from environment variable
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN").expect("Failed to get bot token from environment variable");

    // Send message to chat id
    let url = format!("https://api.telegram.org/bot{}/sendMessage?chat_id={}&text={}", bot_token, chat_id, message);
    let response = reqwest::blocking::get(&url).expect("Failed to send message");
    println!("Telegram response: {}", response.text().unwrap());
}

// Generate a struct that contains a timeslot definition.
// It should contain the following:
// - start time
// - end time
// implement a function that returns the duration of the timeslot in hours
// implement a function that prints the timeslot in the following format:
// "2023-12-01 10:00 - 11:00 (1 h)"
// implement serde::Serialize for the struct
// implement serde::Deserialize for the struct


#[derive(Clone)]
struct Timeslot {
    start: string::String,
    end: string::String,
}

impl Timeslot {
    fn duration(&self) -> i64 {
        let duration = self.end_time() - self.start_time();
        duration.num_hours()
    }

    fn start_time(&self) -> DateTime<Local> {
        let start_time = DateTime::parse_from_rfc3339(&self.start).unwrap().with_timezone(&Local);
        start_time
    }

    fn end_time(&self) -> DateTime<Local> {
        let end_time = DateTime::parse_from_rfc3339(&self.end).unwrap().with_timezone(&Local);
        end_time
    }
}

impl std::fmt::Display for Timeslot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Example output:
        // "2023-12-01 10:00 - 11:00 (1 h)"
        write!(f, "{} - {} ({} h)", self.start_time().format("%Y-%m-%d %H:%M"), self.end_time().format("%H:%M"), self.duration())
    }
}