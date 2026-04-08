//! Reusable SVG icon components
//!
//! Each icon accepts an optional `class` prop for sizing/color overrides
//!
//! Author: aav

// --------------------------------------------------
// external
// --------------------------------------------------
use dioxus::prelude::*;

// --------------------------------------------------
// component
// --------------------------------------------------
#[component]
/// GitHub logo icon
pub(crate) fn GitHubIcon(#[props(default = "w-4 h-4".to_string())] class: String) -> Element {
    rsx! {
        svg {
            class: "{class}",
            fill: "currentColor",
            view_box: "0 0 24 24",
            path {
                d: "M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z",
            }
        }
    }
}

#[component]
/// Globe icon for external links
pub(crate) fn GlobeIcon(#[props(default = "w-4 h-4".to_string())] class: String) -> Element {
    rsx! {
        svg {
            class: "{class}",
            fill: "none",
            view_box: "0 0 24 24",
            stroke_width: "2",
            stroke: "currentColor",
            path {
                stroke_linecap: "round",
                stroke_linejoin: "round",
                d: "M12 21a9.004 9.004 0 0 0 8.716-6.747M12 21a9.004 9.004 0 0 1-8.716-6.747M12 21c2.485 0 4.5-4.03 4.5-9S14.485 3 12 3m0 18c-2.485 0-4.5-4.03-4.5-9S9.515 3 12 3m0 0a8.997 8.997 0 0 1 7.843 4.582M12 3a8.997 8.997 0 0 0-7.843 4.582m15.686 0A11.953 11.953 0 0 1 12 10.5c-2.998 0-5.74-1.1-7.843-2.918m15.686 0A8.959 8.959 0 0 1 21 12c0 .778-.099 1.533-.284 2.253m0 0A17.919 17.919 0 0 1 12 16.5a17.92 17.92 0 0 1-8.716-2.247m0 0A9 9 0 0 1 3 12c0-1.605.42-3.113 1.157-4.418",
            }
        }
    }
}

#[component]
/// Coffee cup icon for donation links
pub(crate) fn CoffeeIcon(#[props(default = "w-4 h-4".to_string())] class: String) -> Element {
    rsx! {
        svg {
            class: "{class}",
            fill: "currentColor",
            view_box: "0 0 24 24",
            path {
                d: "M20.216 6.415l-.132-.666c-.119-.598-.388-1.163-1.001-1.379-.197-.069-.42-.098-.57-.241-.152-.143-.196-.366-.231-.572-.065-.378-.125-.756-.192-1.133-.057-.325-.102-.69-.25-.987-.195-.4-.597-.634-.996-.788a5.723 5.723 0 00-.626-.194c-1-.263-2.05-.36-3.077-.416a25.834 25.834 0 00-3.7.062c-.915.083-1.88.184-2.75.5-.318.116-.646.256-.888.501-.297.302-.393.77-.177 1.146.154.267.415.456.692.58.36.162.737.284 1.123.366 1.075.238 2.189.331 3.287.37 1.218.05 2.437.01 3.65-.118.299-.033.598-.073.896-.119.352-.054.578-.513.474-.834-.124-.383-.457-.531-.834-.473-.466.074-.96.108-1.382.146-1.177.08-2.358.082-3.536.006a22.228 22.228 0 01-1.157-.107c-.086-.01-.18-.025-.258-.036-.243-.036-.484-.08-.724-.13-.111-.027-.111-.185 0-.212h.005c.277-.06.557-.108.838-.147h.002c.131-.009.263-.032.394-.048a25.076 25.076 0 013.426-.12c.674.019 1.347.067 2.017.144l.228.031c.267.04.533.088.798.145.392.085.895.113 1.07.542.055.137.08.288.111.431l.319 1.484a.237.237 0 01-.199.284h-.003c-.037.006-.075.01-.112.015a36.704 36.704 0 01-4.743.295 37.059 37.059 0 01-4.699-.304c-.14-.017-.293-.042-.417-.06-.326-.048-.649-.108-.973-.161-.393-.065-.768-.032-1.123.161-.29.16-.527.404-.675.701-.154.316-.199.66-.267 1-.069.34-.176.707-.135 1.056.087.753.613 1.365 1.37 1.502a39.69 39.69 0 0011.343.376.483.483 0 01.535.53l-.071.697-1.018 9.907c-.041.41-.047.832-.125 1.237-.122.637-.553 1.028-1.182 1.171-.577.131-1.165.185-1.76.222-.91.057-1.823.079-2.736.058-.375-.01-.745-.038-1.12-.072-.109-.01-.22-.021-.316-.056-.2-.072-.4-.2-.494-.396-.1-.21-.075-.46-.042-.69l.065-.401c.073-.452-.25-.876-.722-.876h-.006c-.364 0-.68.264-.735.623-.06.387-.129.77-.185 1.16-.022.149-.039.3-.038.451.002.357.095.706.314.982.226.283.535.46.863.552.646.18 1.326.265 2.006.313.936.069 1.873.098 2.81.064.753-.027 1.51-.088 2.243-.261.959-.226 1.636-.867 1.82-1.842.062-.33.084-.665.12-.999l1.2-11.672a.473.473 0 01.525-.423 4.3 4.3 0 002.832-.776c.862-.627 1.323-1.673 1.168-2.723zM4.67 7.808c.027-.226.092-.444.128-.67.016-.1.037-.15.137-.15h.06c.321 0 .642.015.963.03.315.013.63.036.945.052.103.006.205.016.308.022a.174.174 0 01.164.164l.075.37c.025.122-.08.233-.205.233H5.07a.39.39 0 01-.4-.401zm13.647 2.018a2.86 2.86 0 01-1.474.726l.034-.332.058-.556a.484.484 0 01.537-.435c.406.032.78-.064 1.12-.266.18-.107.391-.08.477.12.093.213-.002.472-.174.631a2.66 2.66 0 01-.578.412z",
            }
        }
    }
}

#[component]
/// Download arrow icon
pub(crate) fn DownloadIcon(#[props(default = "w-5 h-5".to_string())] class: String) -> Element {
    rsx! {
        svg {
            class: "{class}",
            fill: "none",
            view_box: "0 0 24 24",
            stroke_width: "2",
            stroke: "currentColor",
            path {
                stroke_linecap: "round",
                stroke_linejoin: "round",
                d: "M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5M16.5 12 12 16.5m0 0L7.5 12m4.5 4.5V3",
            }
        }
    }
}

#[component]
/// Circled checkmark icon
pub(crate) fn CheckCircleIcon(#[props(default = "w-5 h-5".to_string())] class: String) -> Element {
    rsx! {
        svg {
            class: "{class}",
            fill: "none",
            view_box: "0 0 24 24",
            stroke_width: "2",
            stroke: "currentColor",
            path {
                stroke_linecap: "round",
                stroke_linejoin: "round",
                d: "M9 12.75 11.25 15 15 9.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z",
            }
        }
    }
}

#[component]
/// Clock icon for expiry indicators
pub(crate) fn ClockIcon(#[props(default = "w-5 h-5".to_string())] class: String) -> Element {
    rsx! {
        svg {
            class: "{class}",
            fill: "none",
            view_box: "0 0 24 24",
            stroke_width: "2",
            stroke: "currentColor",
            path {
                stroke_linecap: "round",
                stroke_linejoin: "round",
                d: "M12 6v6h4.5m4.5 0a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z",
            }
        }
    }
}

#[component]
/// Plus icon for add/create actions
pub(crate) fn PlusIcon(#[props(default = "w-4 h-4".to_string())] class: String) -> Element {
    rsx! {
        svg {
            class: "{class}",
            fill: "none",
            view_box: "0 0 24 24",
            stroke_width: "2",
            stroke: "currentColor",
            path {
                stroke_linecap: "round",
                stroke_linejoin: "round",
                d: "M12 4.5v15m7.5-7.5h-15",
            }
        }
    }
}

#[component]
/// Folder icon for directory entries
pub(crate) fn FolderIcon(
    #[props(default = "w-4 h-4 text-accent shrink-0".to_string())] class: String,
) -> Element {
    rsx! {
        svg {
            class: "{class}",
            fill: "none",
            view_box: "0 0 24 24",
            stroke_width: "1.5",
            stroke: "currentColor",
            path {
                stroke_linecap: "round",
                stroke_linejoin: "round",
                d: "M2.25 12.75V12A2.25 2.25 0 0 1 4.5 9.75h15A2.25 2.25 0 0 1 21.75 12v.75m-8.69-6.44-2.12-2.12a1.5 1.5 0 0 0-1.061-.44H4.5A2.25 2.25 0 0 0 2.25 6v12a2.25 2.25 0 0 0 2.25 2.25h15A2.25 2.25 0 0 0 21.75 18V9a2.25 2.25 0 0 0-2.25-2.25h-5.379a1.5 1.5 0 0 1-1.06-.44Z",
            }
        }
    }
}

#[component]
/// X (close) icon for dismiss actions
pub(crate) fn XIcon(#[props(default = "w-4 h-4".to_string())] class: String) -> Element {
    rsx! {
        svg {
            class: "{class}",
            fill: "none",
            view_box: "0 0 24 24",
            stroke_width: "2",
            stroke: "currentColor",
            path {
                stroke_linecap: "round",
                stroke_linejoin: "round",
                d: "M6 18 18 6M6 6l12 12",
            }
        }
    }
}

#[component]
/// File document icon
pub(crate) fn FileIcon(
    #[props(default = "w-4 h-4 text-text-muted shrink-0".to_string())] class: String,
) -> Element {
    rsx! {
        svg {
            class: "{class}",
            fill: "none",
            view_box: "0 0 24 24",
            stroke_width: "1.5",
            stroke: "currentColor",
            path {
                stroke_linecap: "round",
                stroke_linejoin: "round",
                d: "M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m2.25 0H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z",
            }
        }
    }
}

// #[component]
// pub(crate) fn ClipboardIcon(#[props(default = "w-4 h-4".to_string())] class: String) -> Element {
//     rsx! {
//         svg {
//             class: "{class}",
//             fill: "none",
//             view_box: "0 0 24 24",
//             stroke_width: "2",
//             stroke: "currentColor",
//             path {
//                 stroke_linecap: "round",
//                 stroke_linejoin: "round",
//                 d: "M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9.75a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184",
//             }
//         }
//     }
// }
