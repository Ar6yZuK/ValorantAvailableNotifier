use std::time::Duration;

use clap::Parser;
use serde_json::Value;
use tokio::time;

#[derive(clap::Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// Интервал проверки в секундах (по умолчанию 60)
    #[arg(short, long, default_value_t = 60)]
    interval: u64,
}

const URL : &str = "https://valorant.secure.dyn.riotcdn.net/channels/public/x/status/eu.json";

#[tokio::main]
async fn main() {
	let args = Cli::parse();
	
	const MIN_SECS_INTERVAL : u64 = 5;
	let interval_secs = args.interval.max(MIN_SECS_INTERVAL);

	// Переиспользуем клиент для оптимизации TCP/TLS-соединений
    let client = reqwest::Client::new();

    // Настраиваем интервал на 60 секунд
    let mut ticker = time::interval(Duration::from_secs(interval_secs));

    println!("Мониторинг запущен. Проверка каждые {} секунд...\n", interval_secs);

    loop {
        // Первый тик срабатывает немедленно, последующие — ровно через минуту
        ticker.tick().await;

        match fetch_status(&client).await {
            Ok(json) => {
                if let Some(critical_msg) = check_critical_incident(&json) {
                    eprintln!("\n[{}]🚨 КРИТИЧЕСКИЙ СБОЙ ОБНАРУЖЕН! Следующая проверка через {} сек.", current_timestamp(), interval_secs);
                    eprintln!("{}", critical_msg);
                } else {
					println!("[{}] Серверы в норме. Можно играть", current_timestamp());
                    return;
                }
            }
            Err(err) => {
                // Если произошел временный сбой сети, логируем и ждем следующей минуты
                eprintln!("[{}] Ошибка сети при опросе API: {}. Повтор через {} секунд.", current_timestamp(), err, interval_secs);
            }
        }
    }
}
// Запрос и парсинг JSON
async fn fetch_status(client: &reqwest::Client) -> Result<Value, reqwest::Error> {
    client.get(URL).send().await?.json::<Value>().await
}
// Проверка на наличие критических инцидентов
fn check_critical_incident(response: &Value) -> Option<String> {
    let incidents = response["incidents"].as_array()?;

    for incident in incidents {
        let severity = incident["incident_severity"]
            .as_str()
            .or_else(|| incident["severity"].as_str())
            .unwrap_or("info");

        if severity.eq_ignore_ascii_case("critical") {
            let id = incident["id"].as_i64().unwrap_or(0);
            let title = incident["titles"].as_array()
                .and_then(|titles| get_locale_text(titles, "en_US").or_else(|| get_locale_text(titles, "ru_RU")))
                .unwrap_or("Без заголовка");

            return Some(format!("ID: {}\nЗаголовок: {}", id, title));
        }
    }

    None
}
// Простая метка времени для консоли
fn current_timestamp() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

	let secs_of_day = now % 86400;
    let hours = secs_of_day / 3600;
    let minutes = (secs_of_day % 3600) / 60;
    let seconds = secs_of_day % 60;

    format!("{:02}:{:02}:{:02} UTC", hours, minutes, seconds)
}
fn get_locale_text<'a>(items: &'a [Value], target_locale: &str) -> Option<&'a str> {
    for item in items {
        if item["locale"].as_str() == Some(target_locale) {
            return item["content"].as_str();
        }
    }
    items.first().and_then(|item| item["content"].as_str())
}