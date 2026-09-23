use std::time::Duration;

use serde_json::Value;
use tokio::time;

const URL : &str = "https://valorant.secure.dyn.riotcdn.net/channels/public/x/status/eu.json";
#[tokio::main]
async fn main() {
	// Переиспользуем клиент для оптимизации TCP/TLS-соединений
    let client = reqwest::Client::new();

    // Настраиваем интервал на 60 секунд
    let mut ticker = time::interval(Duration::from_secs(60));

    println!("Мониторинг запущен. Проверка каждые 60 секунд...\n");

    loop {
        // Первый тик срабатывает немедленно, последующие — ровно через минуту
        ticker.tick().await;

        match fetch_status(&client).await {
            Ok(json) => {
                if let Some(critical_msg) = check_critical_incident(&json) {
                    eprintln!("\n🚨 КРИТИЧЕСКИЙ СБОЙ ОБНАРУЖЕН! Следующая проверка через 60 сек.");
                    eprintln!("{}", critical_msg);
                } else {
					println!("[{}] Серверы в норме. Можно играть", current_timestamp());
                    return;
                }
            }
            Err(err) => {
                // Если произошел временный сбой сети, логируем и ждем следующей минуты
                eprintln!("[{}] Ошибка сети при опросе API: {}. Повтор через минуту.", current_timestamp(), err);
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
    format!("timestamp: {}", now)
}
fn get_locale_text<'a>(items: &'a [Value], target_locale: &str) -> Option<&'a str> {
    for item in items {
        if item["locale"].as_str() == Some(target_locale) {
            return item["content"].as_str();
        }
    }
    items.first().and_then(|item| item["content"].as_str())
}