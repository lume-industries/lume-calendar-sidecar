use calendar_sidecar::parse_calendar_payload;
use vzglyd_sidecar::{Error, env_var, https_get_text, poll_loop, split_https_url};

fn fetch() -> Result<Vec<u8>, Error> {
    let url = env_var("GCAL_ICS_URL")
        .ok_or_else(|| Error::Io("GCAL_ICS_URL is not set in the sidecar environment".to_string()))?;
    let timezone = env_var("GCAL_TZ").unwrap_or_else(|| "Australia/Melbourne".to_string());
    let (host, path) = split_https_url(&url)?;
    let body = https_get_text(&host, &path)?;
    let payload =
        parse_calendar_payload(&body, now_unix_secs(), &timezone).map_err(Error::Io)?;
    serde_json::to_vec(&payload).map_err(|error| Error::Io(error.to_string()))
}

fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(target_arch = "wasm32")]
fn main() {
    poll_loop(15 * 60, fetch);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("calendar-sidecar is intended for wasm32-wasip1");
}
