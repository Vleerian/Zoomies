use anyhow::Error;
use colored::Colorize;
use inquire::{validator::Validation, CustomType, Text};
use serde::{Deserialize, Serialize};
use serde_xml_rs::from_str;
use spinoff::{spinners, Color, Spinner};
use std::{
    fs::{self, File},
    io::{self, prelude::*, BufRead, BufReader},
    path::Path,
    thread::sleep,
    time::Duration,
};
use ureq::{Agent, AgentBuilder};
use clap::Parser;
use chrono::prelude::*;
use regex::Regex;
use lazy_static::lazy_static;

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

lazy_static! {
    static ref RAIDFILE_RE : Regex = Regex::new(r"region=(?<region>[\w_ ]*).*\((?<timing>[0-9:sm]*)").unwrap();
    static ref TRIGLIST_RE : Regex = Regex::new(r"(?<trigger>[\w _]+)[^@]*?(?:@(?<target>[\w _]+))?[^#]*(?:#(?<comment>[\w _]+))?").unwrap();
}

#[derive(Deserialize)]
struct Region {
    id: String,
    #[serde(alias = "LASTUPDATE")]
    lastupdate: i32,
}

#[derive(Clone, Deserialize, Serialize)]
struct TriggerPrecursor
{
    region: String,
    target: Option<String>,
    updated_ping: bool,
    waiting_ping: bool,
    comment: Option<String>
}

struct Trigger
{
    region : String,
    target : Option<String>,
    lastupdate : i32,
    updated_ping : bool,
    waiting_ping : bool,
    comment : Option<String>
}

impl TriggerPrecursor {
    pub fn from_raidfile(raidfile_line: &String, target : Option<TriggerPrecursor>) -> Option<TriggerPrecursor>
    {
        let Some(capture) = RAIDFILE_RE.captures(raidfile_line.as_str()) else {
            println!("Failed");
            return None;
        };

        let mut comment = (&capture["timing"]).to_string();
        let mut target_str : Option<String> = None;
        let targ_is_some = target.is_some();
        if targ_is_some
        {
            let tmp = target.clone().unwrap().region;
            target_str = Some(tmp.clone());
            comment = format!("Next target, {}! ({})", tmp, target?.comment?)
        }

        Some(TriggerPrecursor {
            region : (&capture["region"]).to_string(),
            target : target_str,
            updated_ping : false,
            waiting_ping : targ_is_some,
            comment : Some(comment)
        })
    }

    pub fn from_triggerlist(triggerlist_line: &String) -> Option<TriggerPrecursor>
    {
        let Some(capture) = TRIGLIST_RE.captures(triggerlist_line.as_str()) else {
            println!("Failed");
            return None;
        };

        let r = match capture.name("target") { Some(reg) => Some(reg.as_str().to_string()), None => None };

        let c = match capture.name("comment") { Some(reg) => Some(format!("Next Target, {}! ", reg.as_str().to_string())), None => None };
        
        Some(TriggerPrecursor {
            region : canonicalize(&capture["trigger"]),
            target : r,
            updated_ping : triggerlist_line.contains("!"),
            waiting_ping : triggerlist_line.contains("$"),
            comment : c
        })
    }
    pub fn from_file(filename: &String) -> Option<Vec<TriggerPrecursor>>
    {
        let trigger_lines = lines_from_file(filename)
            .expect("trigger_list.txt did not exist. Consult README.md for template.");
        // Convert the triggers into trigger precursors
        let mut precursors : Vec<TriggerPrecursor> = Vec::new();
        let mut tmp : Option<TriggerPrecursor> = None;

        // Detect if it's a raidfile
        let raidfile_mode = trigger_lines.first().unwrap().contains("1)");
        info!("Parsing file, mode : {}", if raidfile_mode {"RaidFile"} else {"TriggerList"});

        for mut trigger in trigger_lines {
            trigger = trigger.trim().to_string();
            if trigger == "" {
                continue;
            }
            let precursor : TriggerPrecursor;
            if raidfile_mode
            {
                precursor = match TriggerPrecursor::from_raidfile(&trigger, tmp.clone()) {
                    Some(precursor) => precursor,
                    None => {
                        println!("Failed to parse {}", trigger);
                        continue;
                    }
                };
                // Store the current region if we're looking at the target (triggers are template-none pages)
                if !trigger.contains("template-")
                {
                    tmp = Some(precursor.clone());
                }
            }
            else
            {
                precursor = match TriggerPrecursor::from_triggerlist(&trigger) {
                    Some(precursor) => precursor,
                    None => {
                        println!("Failed to parse {}", trigger);
                        continue;
                    }
                }
            }
            precursors.push(precursor);
        }

        Some(precursors)
    }
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

    // Determine if zoomies should post to webhooks
    let do_pings = args.webhook.is_some();
    let webhook = args.webhook.unwrap_or_else(|| "".to_string());

    // Print the splash
    println!(include_str!("splash.txt"));

    // Set the user agent and initialize the API agent
    let mut api_agent: Agent = AgentBuilder::new()
        .user_agent(&format!(
            "Zoomies/{0} (Developed by nation=Vleerian and nation=Volstrostia;)",
            env!("CARGO_PKG_VERSION")
        ))
        .timeout(Duration::from_secs(15))
        .build();

    // Request main nation if it was not provided in args
    let mut main_nation = args.main_nation.unwrap_or_else(|| Text::new("Main Nation:").prompt().unwrap());
    loop {
        match api_agent.get(format!("https://www.nationstates.net/cgi-bin/api.cgi?nation={}", main_nation).as_str())
            .call() {
                Ok(_) => { break },
                Err(_) => {
                    warn!("Nation {} does not exist.", main_nation);
                    main_nation = Text::new("Main Nation:").prompt().unwrap();       
                }
            }
    }

    api_agent = AgentBuilder::new()
        .user_agent(&format!(
            "Zoomies/{0} (Developed by nation=Vleerian and nation=Volstrostia;); In use by {1}",
            env!("CARGO_PKG_VERSION"),
            main_nation
        ))
        .timeout(Duration::from_secs(15))
        .build();

    let poll_speed = args.poll_speed.unwrap_or_else(|| CustomType::new("Poll Speed (Min 650):")
        .with_default(650)
        .with_validator(validator)
        .prompt().unwrap());

    info!("Zoomies v{} Running as {} at {}ms.", env!("CARGO_PKG_VERSION"), main_nation, poll_speed);
    if do_pings
    {
        let _ = api_agent.post(&webhook).send_json(include!("notify_running.rs"));
    }

    // Load the triggers
    let precursors = TriggerPrecursor::from_file(&trigger_file).unwrap();

    // Get update data
    sleep(Duration::from_millis(poll_speed));
    let lu_banana = get_last_update(&api_agent, "banana").unwrap();
    sleep(Duration::from_millis(poll_speed));
    let lu_wzt = get_last_update(&api_agent, "warzone trinidad").unwrap();
    let update_running: bool = lu_banana.lastupdate > lu_wzt.lastupdate;

    // Fetch and sort trigger data
    let sort_time = ((precursors.len() as u64) * poll_speed) / 1000;
    let spinner_msg = format!("Processing triggers. This will take ~{} seconds.", sort_time);
    let mut spinner = Spinner::new(spinners::Cute, spinner_msg, Color::Yellow);
    let mut trigger_data: Vec<Trigger> = Vec::new();
    for trigger in precursors {
        sleep(Duration::from_millis(poll_speed));
        // Fetch inital last update data and initialize trigger structs
        match get_last_update(&api_agent, &trigger.region) {
            Ok(region) => {
                if update_running && lu_banana.lastupdate < region.lastupdate {
                    warn!("{} has already updated.", region.id);
                } else {
                    trigger_data.push(Trigger {
                        region: region.id,
                        target : trigger.target,
                        lastupdate: region.lastupdate,
                        updated_ping : trigger.updated_ping,
                        waiting_ping : trigger.waiting_ping,
                        comment: trigger.comment
                    })
                }
            }
            Err(_) => warn!("Could not fetch Last Update data for {}.", trigger.region),
        }
    }
    trigger_data.sort_by(|a, b| a.lastupdate.cmp(&b.lastupdate));
    spinner.success("Triggers sorted");

    for trigger in trigger_data {
        let spinner_msg = format!("Waiting for {}...", trigger.region);
        let mut spinner = Spinner::new(spinners::Cute, spinner_msg, Color::Cyan);
        let comment = trigger.comment.unwrap_or_else(|| "".to_string());
        if do_pings && trigger.waiting_ping {
            if trigger.target.is_some()
            {
                let target = trigger.target.unwrap();
                info!("Wait for {} (Trigger {})", target, trigger.region);
                match api_agent.post(&webhook).send_json(include!("waiting_ping.rs")) {
                    Ok(_) => {},
                    Err(_) => warn!("Failed to post wait webhook for {}", trigger.region)
                }
            }
        }
        loop {
            sleep(Duration::from_millis(poll_speed));
            match get_last_update(&api_agent, &trigger.region) {
                Ok(region) => {
                    if region.lastupdate != trigger.lastupdate {
                        beep();
                        let timestring = get_webhook_timestring();
                        let comment = String::from("");
                        let update_message = if comment == "" {
                            format!("{} - {}", trigger.region, comment)
                        } else {
                            format!("UPDATE DETECTED IN {}", trigger.region.to_uppercase())
                        };
                        
                        if do_pings && trigger.updated_ping {
                            match api_agent.post(&webhook).send_json(include!("updated_ping.rs")) {
                                Ok(_) => {},
                                Err(_) => warn!("Failed to post update webhook for {}", trigger.region)
                            }
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
