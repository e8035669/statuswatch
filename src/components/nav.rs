use maud::{html, Markup};

pub fn nav() -> Markup {
    html! {
        nav class="bg-slate-800 text-white px-6 py-3 flex gap-6" {
            a class="font-semibold" href="/" { "Dashboard" }
            a href="/settings/endpoints" { "Endpoints" }
            a href="/history" { "History" }
        }
    }
}
