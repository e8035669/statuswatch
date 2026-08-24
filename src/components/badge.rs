use maud::{html, Markup};

/// Color-coded pill for an `ActiveStatus` value (matched case-insensitively).
pub fn status_badge(status: &str) -> Markup {
    let classes = match status.to_lowercase().as_str() {
        "online" => "bg-green-100 text-green-800",
        "offline" | "abnormal" => "bg-red-100 text-red-800",
        "start" | "stop" => "bg-yellow-100 text-yellow-800",
        _ => "bg-gray-100 text-gray-800",
    };
    let full_classes = format!("inline-block rounded-full px-2 py-0.5 text-xs font-semibold {classes}");
    html! {
        span class=(full_classes) { (status) }
    }
}
