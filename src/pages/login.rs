//! Login page for non-loopback access
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::core::auth;
use crate::routes::AdminRoute;

// --------------------------------------------------
// external
// --------------------------------------------------
use dioxus::prelude::*;

// --------------------------------------------------
// style
// --------------------------------------------------
const CENTER_PAGE_STY: &str =
    crate::classes!("flex", "items-center", "justify-center", "min-h-[60vh]");
const LOGIN_CARD_STY: &str = crate::classes!("card", "w-full", "max-w-sm");
const PAGE_TITLE_STY: &str = crate::classes!("text-xl", "font-bold", "mb-4");
const FORM_STY: &str = crate::classes!("flex", "flex-col", "gap-4");
const ERROR_STY: &str = crate::classes!(
    "mb-4",
    "p-3",
    "bg-danger-bg",
    "border",
    "border-danger/20",
    "rounded-lg",
    "text-danger",
    "text-sm"
);

#[component]
/// Login form for password-protected admin access
pub fn Login() -> Element {
    let nav = navigator();
    let mut password = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut loading = use_signal(|| false);

    let handle_login = move |evt: Event<FormData>| {
        evt.prevent_default();
        loading.set(true);
        error.set(None);

        spawn(async move {
            match auth::login(password()).await {
                Ok(true) => {
                    nav.push(AdminRoute::Dashboard);
                }
                Ok(false) => {
                    error.set(Some("Invalid password".into()));
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    };

    rsx! {
        div { class: CENTER_PAGE_STY,
            div { class: LOGIN_CARD_STY,
                h1 { class: PAGE_TITLE_STY, "Login" }

                if let Some(err) = error() {
                    div { class: ERROR_STY, "{err}" }
                }

                form {
                    class: FORM_STY,
                    onsubmit: handle_login,
                    div {
                        label { class: "label", "Password" }
                        input {
                            r#type: "password",
                            class: "w-full",
                            placeholder: "Enter admin password",
                            value: "{password}",
                            oninput: move |e| password.set(e.value()),
                        }
                    }
                    button {
                        class: "btn-primary",
                        r#type: "submit",
                        disabled: loading(),
                        if loading() { "Signing in..." } else { "Sign In" }
                    }
                }
            }
        }
    }
}
