mod tenant_sessions;

use std::{env, process};
use tenant_sessions::{Client, SessionError};

fn arg(args: &[String], index: usize) -> Result<&str, SessionError> {
    args.get(index).map(String::as_str).ok_or(SessionError::MissingData("command argument"))
}

async fn run() -> Result<(), SessionError> {
    let args: Vec<String> = env::args().collect();
    let command = arg(&args, 1)?;
    let client = Client::from_env()?;
    match command {
        "onboard" if args.len() == 4 => {
            client.onboard(arg(&args, 2)?, arg(&args, 3)?).await?;
            println!("tenant={} room={} ready", args[2], args[3]);
        }
        "token" if args.len() == 4 => {
            println!("{}", client.token(arg(&args, 2)?, arg(&args, 3)?).await?);
        }
        "archive" if args.len() == 6 => {
            let bytes = tokio::fs::read(arg(&args, 5)?).await?;
            let key = client.archive(arg(&args, 2)?, arg(&args, 3)?, arg(&args, 4)?, bytes).await?;
            println!("bucket={} key={key}", args[2]);
        }
        "participants" if args.len() == 3 => {
            println!("{}", client.participants(arg(&args, 2)?).await?);
        }
        "close" if args.len() == 3 => {
            client.close(arg(&args, 2)?).await?;
            println!("room={} closed", args[2]);
        }
        _ => {
            eprintln!("usage: tenant-video-archive onboard TENANT SESSION | token SESSION IDENTITY | archive TENANT SESSION ARTIFACT FILE | participants SESSION | close SESSION");
            process::exit(2);
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        process::exit(1);
    }
}
