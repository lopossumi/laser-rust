use std::fs::File;
use std::thread;
use std::time::Duration;
use chrono::Utc;
use std::io::prelude::*;

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
    // Fetch data from API
    let api_data = fetch_api_data();

    // Read existing data from JSON file. If the file does not exist, create a new empty file.
    // Check if file exists
    if !std::path::Path::new("data.json").exists() {
        // Create new empty file
        let mut file = File::create("data.json").expect("Failed to create file");
        file.write_all("{}".as_bytes()).expect("Failed to write to file");
    }

    let mut file = File::open("data.json").expect("Failed to open file");
    let mut json_data = String::new();
    file.read_to_string(&mut json_data).expect("Failed to read file");

    // Parse JSON data. If the file is empty or parsing fails, create a new empty JSON object.
    let existing_data: serde_json::Value = match serde_json::from_str(&json_data) {
        Ok(data) => data,
        Err(_) => serde_json::from_str("{}").expect("Failed to parse JSON")
    };
    
    // Find new available times
    let new_times = find_new_times(&api_data, &existing_data);

    // Output new times to console
    for time in &new_times {
        println!("New available time: {}", time);
    }

    // Update JSON file with reservations table from API data. Skip other data in the json.
    let updated_data = &api_data["reservations"];
    let mut file = File::create("data.json").expect("Failed to create file");
    file.write_all(updated_data.to_string().as_bytes()).expect("Failed to write to file");

    // Send telegram message with new times
    send_telegram_message(&new_times);
}

fn fetch_api_data() -> serde_json::Value {
    // Fetch data from API and return as string
    let current_time = Utc::now();
    let end_date = current_time + chrono::Duration::days(14);
    let request_url = format!("https://api.hel.fi/respa/v1/resource/axwzr3i57yba/?start={}&end={}&format=json", current_time, end_date);

    // Make API request and return response as a JSON object
    let api_response = reqwest::blocking::get(&request_url).expect("Failed to fetch API data").text().unwrap();
    let api_data: serde_json::Value = serde_json::from_str(&api_response).expect("Failed to parse JSON");
    api_data
}

fn find_new_times(api_data: &serde_json::Value, existing_data: &serde_json::Value) -> Vec<String> {
    // Compare API data with existing data and find new available times.
    // If existing data is empty, return all available times.

// The input from API data looks something like this:
// {
//     "opening_hours": [
//         {
//             "date": "2023-12-01",
//             "opens": "2023-12-01T10:00:00+02:00",
//             "closes": "2023-12-01T14:00:00+02:00"
//         },
//         {
//             "date": "2023-12-02",
//             "opens": "2023-12-02T16:00:00+02:00",
//             "closes": "2023-12-02T19:00:00+02:00"
//         },
//         {
//             "date": "2024-01-01",
//             "opens": null,
//             "closes": null
//         }
//     ],
//     "reservations": [
//         {
//             "begin": "2023-12-01T10:00:00+02:00",
//             "end": "2023-12-01T11:00:00+02:00",
//         },
//         {
//             "begin": "2023-12-01T11:00:00+02:00",
//             "end": "2023-12-01T14:00:00+02:00",
//         }
//     ]
// }
    
        // Get opening hours from API data
        let opening_hours = api_data["opening_hours"].as_array().unwrap();
    
        // Get reservations from API data
        let reservations = api_data["reservations"].as_array().unwrap();
    
        // Get existing reservations from existing data. If existing data is empty, create an empty Vec<Value>.
        let binding = Vec::new();
        let existing_reservations = existing_data["reservations"].as_array().unwrap_or(&binding);
    
        // Create vector for new available times
        let mut new_times: Vec<String> = Vec::new();
    
        // Loop through opening hours
        for opening_hour in opening_hours {
            // Get date from opening hour
            let date = &opening_hour["date"].as_str().unwrap();
    
            // Get opens from opening hour. If opens is null, continue to next opening hour.
            let opens = &opening_hour["opens"];
            if opens.is_null() {
                continue;
            }

            // Get closes from opening hour. If closes is null, skip this opening hour.
            let closes = &opening_hour["closes"];
            if closes.is_null() {
                continue;
            }

            // Loop through reservations
            for reservation in reservations {
                // Get begin from reservation
                let begin = reservation["begin"].as_str().unwrap();
    
                // Get end from reservation
                let end = reservation["end"].as_str().unwrap();
    
                // Check if reservation is on the same date as opening hour
                if begin.starts_with(date) && end.starts_with(date) {
                    // Check if reservation is not in existing reservations
                    if !existing_reservations.contains(&reservation) {
                        // Add reservation to new available times.
                        // The format should look like this:
                        // 2024-01-01: 10:00 - 11:00 (1h)
                        // 2023-01-02: 11:00 - 14:00 (3h)
                        let begin_time = begin.split("T").collect::<Vec<&str>>()[1].split("+").collect::<Vec<&str>>()[0];
                        let end_time = end.split("T").collect::<Vec<&str>>()[1].split("+").collect::<Vec<&str>>()[0];
                        let duration = chrono::DateTime::parse_from_rfc3339(end).unwrap() - chrono::DateTime::parse_from_rfc3339(begin).unwrap();
                        let duration = duration.num_hours();

                        let begin_time_without_seconds = begin_time.chars().take(begin_time.len() - 3).collect::<String>();
                        let end_time_without_seconds = end_time.chars().take(end_time.len() - 3).collect::<String>();
                        let new_time = format!("{}: {} - {} ({}h)", date, begin_time_without_seconds, end_time_without_seconds, duration);
                        new_times.push(new_time);
                    }
                }
            }
        }
    
        // Return new available times
        new_times
}

fn send_telegram_message(new_times: &[String]) {
    // Send telegram message with new available times
    // If there are no new available times, do nothing.
    if new_times.len() == 0 {
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
