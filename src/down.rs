// 死活監視
use lazy_static::lazy_static;
use std::collections::HashMap;
use tokio::time::{Duration, interval, sleep};

lazy_static! {
    static ref WEBHOOK_URL: String =
        std::env::var("DISCORD_WEBHOOK_URL").expect("DISCORD_WEBHOOK_URL must be set");
    static ref DOWN_DETECTOR_URL: Vec<String> = std::env::var("DOWN_DETECTOR_URL")
        .expect("DOWN_DETECTOR_URL must be set")
        .split(',')
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>();
}

async fn send_discord_notification(title: &str, message: &str, color: u32) {
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "username": "Service Down Detected!",
        "embeds": [
            {
                "title": title,
                "description": message,
                "color": color
            }
        ]
    });

    let response = client
        .post(&*WEBHOOK_URL)
        .json(&payload)
        .send()
        .await
        .expect("Failed to send Discord notification");

    if !response.status().is_success() {
        tracing::error!(
            "Failed to send Discord notification: {}",
            response.text().await.unwrap_or_default()
        );
    }
}

async fn down_detect(history: &mut HashMap<String, bool>) {
    for url in DOWN_DETECTOR_URL.iter() {
        let response = reqwest::get(url).await;

        match response {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let message = format!(
                        "Service URL: {}\nStatus Code: {}\nDetail: HTTP Status Code is not success (2xx).",
                        url,
                        resp.status()
                    );
                    history.insert(url.clone(), false);
                    send_discord_notification("Service Down Detected!", &message, 0xff0000).await;
                } else if history.get(url).copied().unwrap_or(true) == false {
                    let message = format!(
                        "Service URL: {}\nStatus Code: {}\nDetail: Service has recovered.",
                        url,
                        resp.status()
                    );
                    history.insert(url.clone(), true);
                    send_discord_notification("Service Recovered!", &message, 0x00ff00).await;
                }
            }
            Err(err) => {
                let status_code: u16 = err.status().map(|s| s.as_u16()).unwrap_or(0);
                let message = format!(
                    "Service URL: {}\nStatus Code: {}\nDetail: {}",
                    url,
                    status_code,
                    err
                );
                history.insert(url.clone(), false);
                send_discord_notification("Service Down Detected!", &message, 0xff0000).await;
            }
        }
    }
}

pub async fn down_detect_loop() {
    let mut down_detect_history: HashMap<String, bool> = HashMap::new();
    let mut ticker = interval(Duration::from_secs(30));
    tracing::info!("Waiting first 30 seconds before starting down detection loop...");
    sleep(Duration::from_secs(30)).await;
    tracing::info!("Down detection loop started.");
    loop {
        ticker.tick().await;
        down_detect(&mut down_detect_history).await;
    }
}
