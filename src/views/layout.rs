use maud::{html, Markup, DOCTYPE};

use crate::components::nav::nav;

pub fn page(title: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " · StatusWatch" }
                link rel="stylesheet" href="/static/css/app.css";
                script src="https://unpkg.com/htmx.org@2.0.4" {}
            }
            body class="bg-slate-50 text-slate-900 min-h-screen" {
                (nav())
                main class="max-w-5xl mx-auto p-6" {
                    (content)
                }
            }
        }
    }
}
