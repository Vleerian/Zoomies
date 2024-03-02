use anyhow::Error;
use colored::Colorize;
use inquire::{validator::Validation, CustomType, Text};
use serde::Deserialize;
use serde_xml_rs::from_str;
use spinoff::{spinners, Color, Spinner};
use std::{
    fs::File,
    io::{self, prelude::*, BufRead, BufReader},
    path::Path,
    thread::sleep,
    time::Duration,
};
use ureq::{Agent, AgentBuilder};
use clap::Parser;
use chrono::prelude::*;

macro_rules! log {
    ($level:ident, $color:expr, $($arg:tt)*) => (println!("[{}] {}", stringify!($level).color($color), format!($($arg)*)));
}

macro_rules! info {
    ($($arg:tt)*) => (log!(INFO, "cyan", $($arg)*));
}

macro_rules! warn {
    ($($arg:tt)*) => (log!(WARN, "yellow", $($arg)*));
}

macro_rules! crit {
    ($($arg:tt)*) => {
        panic!("[{}] {}", "CRIT".color("red"), format!($($arg)*));
    }
}

#[derive(Deserialize)]
struct Region {
    id: String,
    #[serde(alias = "LASTUPDATE")]
    lastupdate: i32,
}

struct Trigger
{
    region: String,
    lastupdate: i32,
    ping: bool,
    comment: Option<String>
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args
{
    // Optional main nation to skip zoomies querying for it
    #[arg(short, long)]
    main_nation: Option<String>,

    // Optional poll speed to skip zoomies querying for it
    #[arg(short, long)]
    poll_speed: Option<u64>,

    // Optional filename for trigger list
    #[arg(long)]
    filepath: Option<String>,

    // Optional Webhook URL, pings will only happen if this is present
    #[arg(long)]
    webhook: Option<String>,

    // Optional: Raidfile mode, loads a Quickdraw raidfile instead of triggerlist
    #[arg(long)]
    raidfile: Option<bool>
}

// Yoinked from https://stackoverflow.com/questions/30801031/read-a-file-and-get-an-array-of-strings
fn lines_from_file(filename: impl AsRef<Path>) -> io::Result<Vec<String>> {
    BufReader::new(File::open(filename)?).lines().collect()
}

fn canonicalize(string: &str) -> String {
    let mut output = String::from(string);
    output.make_ascii_lowercase();
    return str::replace(output.as_str().trim(), " ", "_");
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
    print!("\x07")
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
    crit!("Created {filename}. Please edit it to contain the names of your trigger regions.");
}

/// Creates a timestamp. It is moved to EST, since that's the NS servers' timezone
fn get_webhook_timestring() -> String {
    let offset = FixedOffset::east_opt((5 * 60 * 60)*-1).unwrap();
    let now = Utc::now().with_timezone(&offset);

    let (is_pm, hour) = now.hour12();
    format!(
        "{:02}:{:02}:{:02} {}",
        hour,
        now.minute(),
        now.second(),
        if is_pm { "PM" } else { "AM" }
    )
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

    let args = Args::parse();
    
    // Check that trigger_list exists
    let trigger_file = args.filepath.unwrap_or("trigger_list.txt".to_string());
    let _ = check_for_file(trigger_file.as_str()) || create_file(trigger_file.as_str());

    // TODO: Move splash to separate rs file
    // Print the splash
    println!(r" _  _______  _______  _______  _______ _________ _______  _______  _ ");
    println!(r"( )/ ___   )(  ___  )(  ___  )(       )\__   __/(  ____ \(  ____ \( )");
    println!(r" \|\/   )  || (   ) || (   ) || () () |   ) (   | (    \/| (    \/ \|");
    println!(r"       /   )| |   | || |   | || || || |   | |   | (__    | (_____    ");
    println!(r"      /   / | |   | || |   | || |(_)| |   | |   |  __)   (_____  )   ");
    println!(r"     /   /  | |   | || |   | || |   | |   | |   | (            ) |   ");
    println!(r"    /   (_/\| (___) || (___) || )   ( |___) (___| (____/\/\____) |   ");
    println!(r"   (_______/(_______)(_______)|/     \|\_______/(_______/\_______)   ");
    println!("--===ᕕ( ᐛ )ᕗ");

    // Request main nation if it was not provided in args
    let main_nation = args.main_nation.unwrap_or_else(|| Text::new("Main Nation:").prompt().unwrap());

    let poll_speed = CustomType::new("Poll Speed (Min 650):")
        .with_default(650)
        .with_validator(validator)
        .prompt()
        .unwrap());
    info!("Running as {} at {}ms.", main_nation, poll_speed);

    // Set the user agent and initialize the API agent
    let user_agent = format!(
        "Zoomies/{0} (Developed by nation=Vleerian and nation=Volstrostia; In use by nation={1})",
        env!("CARGO_PKG_VERSION"),
        main_nation
    );
    let api_agent: Agent = AgentBuilder::new()
        .user_agent(&user_agent)
        .timeout(Duration::from_secs(15))
        .build();

    // Load the triggers
    let mut triggers = lines_from_file(trigger_file)
        .expect("trigger_list.txt did not exist. Consult README.md for template.");
    // If it is a raidfile, some parsing is required
    if args.raidfile.is_some() && args.raidfile.unwrap()
    {
        let mut tmp : Vec<String> = Vec::new();
        for trigger in triggers {
            if !trigger.contains("http")
            {
                continue;
            }

            let mut target = trigger
                .split(&['=', ' ']).nth_back(1)
                .unwrap().replace("^", "")
                .to_string();
            if !trigger.contains("template-")
            {
                target.push('!');
            }
            tmp.push(target);
        }
        triggers = tmp;
    }

    // Determine if zoomies should post to webhooks
    let do_pings = args.webhook.is_some();
    let webhook = args.webhook.unwrap_or_else(|| "".to_string());

    // Get update data
    sleep(Duration::from_millis(poll_speed));
    let lu_banana = get_last_update(&api_agent, "banana").unwrap();
    sleep(Duration::from_millis(poll_speed));
    let lu_wzt = get_last_update(&api_agent, "warzone trinidad").unwrap();
    let update_running: bool = lu_banana.lastupdate > lu_wzt.lastupdate;

    // Fetch and sort trigger data
    let sort_time = ((triggers.len() as u64) * poll_speed) / 1000;
    let spinner_msg = format!("Processing triggers. This will take ~{} seconds.", sort_time);
    let mut spinner = Spinner::new(spinners::Cute, spinner_msg, Color::Yellow);
    let mut trigger_data: Vec<Trigger> = Vec::new();
    for mut trigger in triggers {
        sleep(Duration::from_millis(poll_speed));
        let mut ping : bool = false;
        
        // Comment parsing
        let clone = trigger.clone();
        let mut split = clone.split('#');
        trigger = split.next().unwrap().to_string();
        let comment = split.next().map_or(None, |s| Some(s.to_string()));
        
        // Detect if a trigger wants to be pinged
        if trigger.contains("!") {
            trigger = trigger.replace("!", "");
            ping = true;
        }

        // Fetch inital last update data and initialize trigger structs
        match get_last_update(&api_agent, &trigger) {
            Ok(region) => {
                if update_running && lu_banana.lastupdate < region.lastupdate {
                    warn!("{} has already updated.", region.id);
                } else {
                    trigger_data.push(Trigger {
                        region: region.id,
                        lastupdate: region.lastupdate,
                        ping: ping,
                        comment: comment
                    })
                }
            }
            Err(_) => warn!("Could not fetch Last Update data for {}.", trigger),
        }
    }
    trigger_data.sort_by(|a, b| a.lastupdate.cmp(&b.lastupdate));
    spinner.success("Triggers sorted");

    for trigger in trigger_data {
        let spinner_msg = format!("Waiting for {}...", trigger.region);
        let mut spinner = Spinner::new(spinners::Cute, spinner_msg, Color::Cyan);
        loop {
            sleep(Duration::from_millis(poll_speed));
            match get_last_update(&api_agent, &trigger.region) {
                Ok(region) => {
                    if region.lastupdate != trigger.lastupdate {
                        beep();
                        let timestring = get_webhook_timestring();
                        let update_message = if trigger.comment.is_some() {
                            format!("{} - {}", trigger.region, trigger.comment.unwrap())
                        } else {
                            format!("UPDATE DETECTED IN {}", trigger.region.to_uppercase())
                        };
                        
                        if trigger.ping && do_pings {
                            let _ = api_agent.post(&webhook).send_json(include!("webhook.rs"));
                        }

                        let success_msg = format!("{} {}", timestring, update_message).green().bold();
                        spinner.success(&success_msg);
                        break;
                    }
                }
                Err(_) => warn!("Fetch failed!"),
            }
        }
    }
}
