use anyhow::Error;
use inquire::{validator::Validation, CustomType, Text};
use serde::Deserialize;
use serde_xml_rs::from_str;
use spinoff::{spinners, Color, Spinner};
use std::io::prelude::*;
use std::time::Duration;
use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    path::Path,
    thread::sleep,
};
use ureq::{Agent, AgentBuilder};

pub const LOG_THRESHHOLD: i32 = 30;

macro_rules! log {
    ($level:ident, $($arg:tt)*) => (println!("[{}] {}", stringify!($level), format!($($arg)*)));
}

macro_rules! info {
    ($($arg:tt)*) => (log!(INFO, $($arg)*));
}

macro_rules! warn {
    ($($arg:tt)*) => (log!(WARN, $($arg)*));
}

#[derive(Deserialize)]
struct Region {
    id: String,
    #[serde(alias = "LASTUPDATE")]
    lastupdate: i32,
}

// Yoinked from https://stackoverflow.com/questions/30801031/read-a-file-and-get-an-array-of-strings
fn lines_from_file(filename: impl AsRef<Path>) -> io::Result<Vec<String>> {
    BufReader::new(File::open(filename)?).lines().collect()
}

fn canonicalize(string: &str) -> String {
    let mut output = String::from(string);
    output.make_ascii_lowercase();
    return str::replace(output.as_str(), " ", "_");
}

fn get_last_update(agent: &Agent, region: &str) -> Result<Region, Error> {
    let url = format!(
        "https://www.nationstates.net/cgi-bin/api.cgi?region={}&q=lastupdate",
        canonicalize(region)
    );

    let response = agent.get(&url).call()?.into_string()?;

    let response: Region = from_str(&response)?;
    Ok(response)
}

#[cfg(target_family = "windows")]
fn beep() {
    win_beep::beep_with_hz_and_millis(800, 200);
}

#[cfg(target_family = "unix")]
fn beep() {
    print!(r"\x07")
}

/// Returns true if the file exists, and false if it does not
fn check_for_file(filename: &str) -> bool {
    Path::new(filename).exists()
}

/// Creates a file, then aborts execution to prevent using this file as-is
fn create_file(filename: &str) -> ! {
    let mut file = File::create(filename).expect("Failed to create file");
    file.write_all(b"region_one\nregion_two\nregion_three")
        .expect("Failed to write file");
    panic!("Created {filename}. Please edit it to contain the names of your trigger regions.");
}

fn main() {
    // The validator used to ensure that the provided poll speed is valid
    let validator = |input: &u64| {
        if input < &650 {
            Ok(Validation::Invalid("Poll speed minimum is 650ms".into()))
        } else {
            Ok(Validation::Valid)
        }
    };

    // Check that trigger_list exists
    let _ = check_for_file("trigger_list.txt") || create_file("trigger_list.txt");

    println!(r" _  _______  _______  _______  _______ _________ _______  _______  _ ");
    println!(r"( )/ ___   )(  ___  )(  ___  )(       )\__   __/(  ____ \(  ____ \( )");
    println!(r" \|\/   )  || (   ) || (   ) || () () |   ) (   | (    \/| (    \/ \|");
    println!(r"       /   )| |   | || |   | || || || |   | |   | (__    | (_____    ");
    println!(r"      /   / | |   | || |   | || |(_)| |   | |   |  __)   (_____  )   ");
    println!(r"     /   /  | |   | || |   | || |   | |   | |   | (            ) |   ");
    println!(r"    /   (_/\| (___) || (___) || )   ( |___) (___| (____/\/\____) |   ");
    println!(r"   (_______/(_______)(_______)|/     \|\_______/(_______/\_______)   ");
    println!("--===ᕕ( ᐛ )ᕗ");

    let main_nation = Text::new("Main Nation:").prompt().unwrap();

    let poll_speed = CustomType::new("Poll Speed (Min 650):")
        .with_validator(validator)
        .prompt()
        .unwrap();

    let user_agent = format!(
        "Zoomies/{0} (Developed by nation=Vleerian and nation=Volstrostia; In use by nation={1})",
        env!("CARGO_PKG_VERSION"),
        main_nation
    );

    let api_agent: Agent = AgentBuilder::new()
        .user_agent(&user_agent)
        .timeout(Duration::from_secs(15))
        .build();

    let triggers = lines_from_file("trigger_list.txt")
        .expect("trigger_list.txt did not exist. Consult README.md for template.");

    sleep(Duration::from_millis(poll_speed));
    let lu_banana = get_last_update(&api_agent, "banana").unwrap();
    sleep(Duration::from_millis(poll_speed));
    let lu_wzt = get_last_update(&api_agent, "warzone trinidad").unwrap();
    let update_running: bool = lu_banana.lastupdate > lu_wzt.lastupdate;

    // Fetch and sort trigger data
    let sort_time = ((triggers.len() as u64) * poll_speed) / 1000;
    info!("Sorting triggers. This will take ~{} seconds.", sort_time);
    let mut trigger_data: Vec<Region> = Vec::new();
    for trigger in triggers {
        sleep(Duration::from_millis(poll_speed));
        match get_last_update(&api_agent, &trigger) {
            Ok(region) => {
                if update_running && lu_banana.lastupdate < region.lastupdate {
                    warn!("{} has already updated.", region.id);
                } else {
                    trigger_data.push(region)
                }
            }
            Err(_) => warn!("Could not fetch Last Update data for {}.", trigger),
        }
    }
    trigger_data.sort_by(|a, b| a.lastupdate.cmp(&b.lastupdate));
    info!("Triggers sorted.");

    for trigger in trigger_data {
        // info!("Waiting for {}", trigger.id);
        let spinner_msg = format!("Waiting for {}...", trigger.id);
        let mut spinner = Spinner::new(spinners::Clock, spinner_msg, Color::Yellow);
        loop {
            sleep(Duration::from_millis(poll_speed));
            match get_last_update(&api_agent, &trigger.id) {
                Ok(region) => {
                    if region.lastupdate != trigger.lastupdate {
                        beep();
                        // println!("{}\n{} HAS UPDATED\n{}", banner, region.id, banner);
                        let success_msg = format!("{} HAS UPDATED", region.id);
                        spinner.success(&success_msg);
                        break;
                    }
                }
                Err(_) => warn!("Fetch failed!"),
            }
        }
    }
}
