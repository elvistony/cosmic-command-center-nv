// SPDX-License-Identifier: MPL-2.0

use crate::config::Config;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::widget::{column, row, space};
use cosmic::iced::window::Id;
use cosmic::iced::{Alignment, Length, Limits, Subscription};
use cosmic::prelude::*;
use cosmic::widget::{self, list_column};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveSection {
    Graphics,
    Tailscale,
    Surfshark,
}

#[derive(Default)]
pub struct AppModel {
    core: cosmic::Core,
    popup: Option<Id>,
    config: Config,

    // App state
    active_section: Option<ActiveSection>,
    current_mode: String,
    status: String,

    // Feature states
    tailscale_on: bool,
    tailscale_exit_node: String,
    surfshark_on: bool,
    surfshark_location: String,
    owly_on: bool,
    dolby_atmos_on: bool,
    mic_noise_on: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    UpdateConfig(Config),
    ToggleSection(ActiveSection),

    // Graphics
    GraphicsModeFetched(Result<String, String>),
    SetGraphicsMode(String),
    GraphicsModeChanged(Result<(), String>),

    // Tailscale
    ToggleTailscale,
    SelectExitNode(String),

    // Surfshark
    ToggleSurfshark,
    SelectSurfsharkLocation(String),
    VpnStatusFetched(Option<String>),

    // Owly AirDrop
    ToggleOwly,

    // Audio Effects
    ToggleDolbyAtmos,
    ToggleMicNoise,
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "io.github.unfoundry.cosmic-control-center-nv";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let config = cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
            .map(|context| match Config::get_entry(&context) {
                Ok(config) => config,
                Err((_errors, config)) => config,
            })
            .unwrap_or_default();

        let app = AppModel {
            core,
            tailscale_exit_node: config
                .tailscale_exit_nodes
                .first()
                .cloned()
                .unwrap_or_else(|| "None (Direct)".to_string()),
            surfshark_location: config
                .surfshark_locations
                .first()
                .cloned()
                .unwrap_or_else(|| "Delhi".to_string()),
            config,
            current_mode: "loading...".to_string(),
            ..Default::default()
        };

        let init_task = Task::perform(
            async {
                match tokio::task::spawn_blocking(|| {
                    Command::new("system76-power").arg("graphics").output()
                })
                .await
                {
                    Ok(Ok(output)) if output.status.success() => {
                        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
                    }
                    Ok(Ok(output)) => {
                        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
                    }
                    Ok(Err(e)) => Err(e.to_string()),
                    Err(e) => Err(e.to_string()),
                }
            },
            Message::GraphicsModeFetched,
        )
        .map(Into::into);

        (app, init_task)
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn view(&self) -> Element<'_, Self::Message> {
        self.core
            .applet
            .icon_button("display-symbolic")
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        let mut list = list_column();

        // ==========================================
        // 1. GRAPHICS SECTION
        // ==========================================
        let mode_lower = self.current_mode.to_lowercase();
        let current_icon_name = if mode_lower.contains("integrated") {
            "battery-symbolic"
        } else if mode_lower.contains("compute") {
            "video-display-symbolic"
        } else if mode_lower.contains("hybrid") {
            "power-profile-balanced-symbolic"
        } else if mode_lower.contains("nvidia") {
            "power-profile-performance-symbolic"
        } else {
            "display-symbolic"
        };

        let is_gfx_open = self.active_section == Some(ActiveSection::Graphics);
        let chevron_gfx = if is_gfx_open {
            "pan-up-symbolic"
        } else {
            "pan-down-symbolic"
        };

        let gfx_header = row![
            widget::icon::from_name(current_icon_name)
                .size(16)
                .symbolic(true),
            column![
                widget::text::body("Graphics Mode"),
                widget::text::caption(if self.status.is_empty() {
                    &self.current_mode
                } else {
                    &self.status
                }),
            ]
            .spacing(1)
            .width(Length::Fill),
            widget::icon::from_name(chevron_gfx)
                .size(16)
                .symbolic(true),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .width(Length::Fill);

        list = list.add(
            widget::list::button(gfx_header)
                .on_press(Message::ToggleSection(ActiveSection::Graphics)),
        );

        if is_gfx_open {
            let is_int = self.current_mode.eq_ignore_ascii_case("integrated");
            let is_cmp = self.current_mode.eq_ignore_ascii_case("compute");
            let is_hyb = self.current_mode.eq_ignore_ascii_case("hybrid");
            let is_nvd = self.current_mode.eq_ignore_ascii_case("nvidia");

            let make_gfx_subitem = |name: &'static str, desc: &'static str, icon: &'static str, is_active: bool| {
                let mut r = row![
                    space().width(16),
                    widget::icon::from_name(icon).size(14).symbolic(true),
                    column![
                        widget::text::body(name),
                        widget::text::caption(desc),
                    ]
                    .spacing(1)
                    .width(Length::Fill),
                ]
                .spacing(10)
                .align_y(Alignment::Center)
                .width(Length::Fill);

                if is_active {
                    r = r.push(
                        widget::icon::from_name("emblem-ok-symbolic")
                            .size(14)
                            .symbolic(true),
                    );
                }
                r
            };

            list = list
                .add(
                    widget::list::button(make_gfx_subitem(
                        "Integrated",
                        "Battery saving (Intel graphics)",
                        "battery-symbolic",
                        is_int,
                    ))
                    .selected(is_int)
                    .on_press(Message::SetGraphicsMode("integrated".into())),
                )
                .add(
                    widget::list::button(make_gfx_subitem(
                        "Compute",
                        "NVIDIA active for CUDA only",
                        "video-display-symbolic",
                        is_cmp,
                    ))
                    .selected(is_cmp)
                    .on_press(Message::SetGraphicsMode("compute".into())),
                )
                .add(
                    widget::list::button(make_gfx_subitem(
                        "Hybrid",
                        "Automatic GPU switching",
                        "power-profile-balanced-symbolic",
                        is_hyb,
                    ))
                    .selected(is_hyb)
                    .on_press(Message::SetGraphicsMode("hybrid".into())),
                )
                .add(
                    widget::list::button(make_gfx_subitem(
                        "NVIDIA",
                        "Dedicated GPU performance",
                        "power-profile-performance-symbolic",
                        is_nvd,
                    ))
                    .selected(is_nvd)
                    .on_press(Message::SetGraphicsMode("nvidia".into())),
                );
        }

        // ==========================================
        // 2. TAILSCALE (Accordion + Toggler)
        // ==========================================
        let is_ts_open = self.active_section == Some(ActiveSection::Tailscale);
        let chevron_ts = if is_ts_open {
            "pan-up-symbolic"
        } else {
            "pan-down-symbolic"
        };

        let ts_chevron_btn = cosmic::widget::button::icon(widget::icon::from_name(chevron_ts).size(14))
            .class(cosmic::theme::Button::Transparent)
            .on_press(Message::ToggleSection(ActiveSection::Tailscale));

        let ts_row = row![
            widget::icon::from_name("network-vpn-symbolic")
                .size(16)
                .symbolic(true),
            column![
                widget::text::body("Tailscale"),
                widget::text::caption(if self.tailscale_on {
                    format!("ON — Exit: {}", self.tailscale_exit_node)
                } else {
                    "OFF".to_string()
                }),
            ]
            .spacing(1)
            .width(Length::Fill),
            ts_chevron_btn,
            widget::toggler(self.tailscale_on).on_toggle(|_| Message::ToggleTailscale),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .width(Length::Fill);

        list = list.add(ts_row);

        if is_ts_open {
            for node in &self.config.tailscale_exit_nodes {
                let node_clone = node.clone();
                let is_selected = self.tailscale_exit_node == node_clone;

                let icon_name = if node_clone.contains("None") {
                    "network-wired-symbolic"
                } else if node_clone.contains("cloud") {
                    "weather-few-clouds-symbolic"
                } else {
                    "network-server-symbolic"
                };

                let mut node_row = row![
                    space().width(16),
                    widget::icon::from_name(icon_name).size(14).symbolic(true),
                    widget::text::body(format!("Exit Node: {}", node_clone)).width(Length::Fill),
                ]
                .spacing(10)
                .align_y(Alignment::Center)
                .width(Length::Fill);

                if is_selected {
                    node_row = node_row.push(
                        widget::icon::from_name("emblem-ok-symbolic")
                            .size(14)
                            .symbolic(true),
                    );
                }

                list = list.add(
                    widget::list::button(node_row)
                        .selected(is_selected)
                        .on_press(Message::SelectExitNode(node_clone)),
                );
            }
        }

        // ==========================================
        // 3. SURFSHARK VPN (Accordion + Toggler)
        // ==========================================
        let is_ss_open = self.active_section == Some(ActiveSection::Surfshark);
        let chevron_ss = if is_ss_open {
            "pan-up-symbolic"
        } else {
            "pan-down-symbolic"
        };

        let ss_chevron_btn = cosmic::widget::button::icon(widget::icon::from_name(chevron_ss).size(14))
            .class(cosmic::theme::Button::Transparent)
            .on_press(Message::ToggleSection(ActiveSection::Surfshark));

        let ss_row = row![
            widget::icon::from_name("security-high-symbolic")
                .size(16)
                .symbolic(true),
            column![
                widget::text::body("VPN"),
                widget::text::caption(if self.surfshark_on {
                    format!("ON — {}", self.surfshark_location)
                } else {
                    "OFF".to_string()
                }),
            ]
            .spacing(1)
            .width(Length::Fill),
            ss_chevron_btn,
            widget::toggler(self.surfshark_on).on_toggle(|_| Message::ToggleSurfshark),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .width(Length::Fill);

        list = list.add(ss_row);

        if is_ss_open {
            for loc in &self.config.surfshark_locations {
                let loc_clone = loc.clone();
                let is_selected = self.surfshark_location == loc_clone;

                let mut loc_row = row![
                    space().width(16),
                    widget::icon::from_name("find-location-symbolic").size(14).symbolic(true),
                    widget::text::body(format!("Location: {}", loc_clone)).width(Length::Fill),
                ]
                .spacing(10)
                .align_y(Alignment::Center)
                .width(Length::Fill);

                if is_selected {
                    loc_row = loc_row.push(
                        widget::icon::from_name("emblem-ok-symbolic")
                            .size(14)
                            .symbolic(true),
                    );
                }

                list = list.add(
                    widget::list::button(loc_row)
                        .selected(is_selected)
                        .on_press(Message::SelectSurfsharkLocation(loc_clone)),
                );
            }
        }

        // ==========================================
        // 4. AIRDROP (OWLY)
        // ==========================================
        let owly_row = row![
            widget::icon::from_name("network-wireless-symbolic")
                .size(16)
                .symbolic(true),
            column![
                widget::text::body("AirDrop"),
                widget::text::caption(if self.owly_on {
                    "Active (Radio Up)"
                } else {
                    "Disabled"
                }),
            ]
            .spacing(1)
            .width(Length::Fill),
            widget::toggler(self.owly_on).on_toggle(|_| Message::ToggleOwly),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .width(Length::Fill);

        list = list.add(owly_row);

        // ==========================================
        // 5. AUDIO EFFECTS (Dolby Atmos & Mic Noise Dual Action Buttons)
        // ==========================================
        let (dolby_status, dolby_class) = if self.dolby_atmos_on {
            ("ON", cosmic::theme::Button::Suggested)
        } else {
            ("OFF", cosmic::theme::Button::Standard)
        };

        let dolby_icon = if self.dolby_atmos_on {
            widget::icon::from_name("audio-speakers-symbolic")
                .size(16)
                .symbolic(false)
        } else {
            widget::icon::from_name("audio-speakers-symbolic")
                .size(16)
                .symbolic(true)
        };

        let dolby_btn = cosmic::widget::button::custom(
            row![
                dolby_icon,
                column![
                    widget::text::body("Dolby Atmos"),
                    widget::text::caption(dolby_status),
                ]
                .spacing(1)
                .width(Length::Fill),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .width(Length::Fill),
        )
        .padding([8, 12])
        .class(dolby_class)
        .on_press(Message::ToggleDolbyAtmos)
        .width(Length::FillPortion(1));

        let (mic_status, mic_class) = if self.mic_noise_on {
            ("ON", cosmic::theme::Button::Suggested)
        } else {
            ("OFF", cosmic::theme::Button::Standard)
        };

        let mic_icon = if self.mic_noise_on {
            widget::icon::from_name("audio-input-microphone-symbolic")
                .size(16)
                .symbolic(false)
        } else {
            widget::icon::from_name("audio-input-microphone-symbolic")
                .size(16)
                .symbolic(true)
        };

        let mic_btn = cosmic::widget::button::custom(
            row![
                mic_icon,
                column![
                    widget::text::body("Mic Noise"),
                    widget::text::caption(mic_status),
                ]
                .spacing(1)
                .width(Length::Fill),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .width(Length::Fill),
        )
        .padding([8, 12])
        .class(mic_class)
        .on_press(Message::ToggleMicNoise)
        .width(Length::FillPortion(1));

        let audio_row = row![dolby_btn, mic_btn]
            .spacing(8)
            .align_y(Alignment::Center)
            .width(Length::Fill);

        list = list.add(audio_row);

        self.core.applet.popup_container(list).into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        self.core()
            .watch_config::<Config>(Self::APP_ID)
            .map(|update| Message::UpdateConfig(update.config))
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::UpdateConfig(config) => {
                self.config = config;
            }
            Message::ToggleSection(section) => {
                if self.active_section == Some(section) {
                    self.active_section = None;
                } else {
                    self.active_section = Some(section);
                }
            }
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(380.0)
                        .min_width(320.0)
                        .min_height(200.0)
                        .max_height(700.0);

                    let refresh = Task::perform(
                        async {
                            match tokio::task::spawn_blocking(|| {
                                Command::new("system76-power").arg("graphics").output()
                            })
                            .await
                            {
                                Ok(Ok(output)) if output.status.success() => {
                                    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
                                }
                                Ok(Ok(output)) => {
                                    Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
                                }
                                Ok(Err(e)) => Err(e.to_string()),
                                Err(e) => Err(e.to_string()),
                            }
                        },
                        Message::GraphicsModeFetched,
                    )
                    .map(Into::into);

                    let refresh_vpn = Task::perform(
                        async {
                            tokio::task::spawn_blocking(|| {
                                let output = Command::new("nmcli")
                                    .args(["-t", "-f", "NAME,TYPE", "connection", "show", "--active"])
                                    .output()
                                    .ok()?;
                                if !output.status.success() {
                                    return None;
                                }
                                let stdout = String::from_utf8_lossy(&output.stdout);
                                for line in stdout.lines() {
                                    let parts: Vec<&str> = line.split(':').collect();
                                    if parts.len() >= 2 && parts[1] == "wireguard" {
                                        let name = parts[0];
                                        if ["delhi", "london", "chicago"].contains(&name.to_lowercase().as_str()) {
                                            return Some(name.to_string());
                                        }
                                    }
                                }
                                None
                            })
                            .await
                            .unwrap_or(None)
                        },
                        Message::VpnStatusFetched,
                    )
                    .map(Into::into);

                    Task::batch(vec![get_popup(popup_settings), refresh, refresh_vpn])
                };
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
            Message::GraphicsModeFetched(res) => match res {
                Ok(mode) => {
                    self.current_mode = mode;
                }
                Err(err) => {
                    self.current_mode = "unknown".into();
                    self.status = format!("Err: {}", err);
                }
            },
            Message::SetGraphicsMode(mode) => {
                self.status = format!("Switching to {}...", mode);
                let mode_clone = mode.clone();
                return Task::perform(
                    async move {
                        match tokio::task::spawn_blocking(move || {
                            Command::new("pkexec")
                                .args(["system76-power", "graphics", &mode_clone])
                                .status()
                        })
                        .await
                        {
                            Ok(Ok(status)) if status.success() => Ok(()),
                            Ok(Ok(status)) => Err(format!("Command exited with {}", status)),
                            Ok(Err(e)) => Err(e.to_string()),
                            Err(e) => Err(e.to_string()),
                        }
                    },
                    Message::GraphicsModeChanged,
                )
                .map(Into::into);
            }
            Message::GraphicsModeChanged(res) => match res {
                Ok(()) => {
                    self.status = "Reboot required".into();
                    return Task::perform(
                        async {
                            match tokio::task::spawn_blocking(|| {
                                Command::new("system76-power").arg("graphics").output()
                            })
                            .await
                            {
                                Ok(Ok(output)) if output.status.success() => {
                                    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
                                }
                                Ok(Ok(output)) => {
                                    Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
                                }
                                Ok(Err(e)) => Err(e.to_string()),
                                Err(e) => Err(e.to_string()),
                            }
                        },
                        Message::GraphicsModeFetched,
                    )
                    .map(Into::into);
                }
                Err(e) => {
                    self.status = format!("Failed: {}", e);
                }
            },

            Message::VpnStatusFetched(active_loc) => {
                if let Some(loc) = active_loc {
                    self.surfshark_on = true;
                    self.surfshark_location = loc;
                } else {
                    self.surfshark_on = false;
                }
            }

            // Tailscale Handlers
            Message::ToggleTailscale => {
                self.tailscale_on = !self.tailscale_on;
                let enable = self.tailscale_on;
                if enable {
                    self.active_section = Some(ActiveSection::Tailscale);
                }
                return Task::perform(
                    async move {
                        let _ = tokio::task::spawn_blocking(move || {
                            let arg = if enable { "up" } else { "down" };
                            let res = Command::new("tailscale").arg(arg).status();
                            if let Ok(status) = res {
                                if !status.success() {
                                    // Fallback to pkexec if operator permission isn't set yet
                                    let _ = Command::new("pkexec")
                                        .args(["tailscale", arg])
                                        .status();
                                }
                            }
                        })
                        .await;
                    },
                    |_| Message::UpdateConfig(Config::default()),
                )
                .map(Into::into);
            }
            Message::SelectExitNode(node) => {
                self.tailscale_exit_node = node.clone();
                return Task::perform(
                    async move {
                        let _ = tokio::task::spawn_blocking(move || {
                            let exit_target = if node.contains("None") {
                                ""
                            } else if node.contains("cloud-pc") {
                                "100.120.114.52"
                            } else if node.contains("docker-host-tertiary") {
                                "100.86.60.61"
                            } else if node.contains("docker-host") {
                                "100.115.130.80"
                            } else {
                                node.as_str()
                            };

                            let exit_flag = format!("--exit-node={}", exit_target);
                            let res = Command::new("tailscale")
                                .args(["set", &exit_flag])
                                .status();
                            if let Ok(status) = res {
                                if !status.success() {
                                    let _ = Command::new("pkexec")
                                        .args(["tailscale", "set", &exit_flag])
                                        .status();
                                }
                            }
                        })
                        .await;
                    },
                    |_| Message::UpdateConfig(Config::default()),
                )
                .map(Into::into);
            }

            // Surfshark / WireGuard Handlers
            Message::ToggleSurfshark => {
                self.surfshark_on = !self.surfshark_on;
                let enable = self.surfshark_on;
                if enable {
                    self.active_section = Some(ActiveSection::Surfshark);
                }
                let loc = self.surfshark_location.clone();
                return Task::perform(
                    async move {
                        let _ = tokio::task::spawn_blocking(move || {
                            if enable {
                                // Deactivate any other active vpn profiles first
                                for other in ["Delhi", "London", "Chicago"] {
                                    if !other.eq_ignore_ascii_case(&loc) {
                                        let _ = Command::new("nmcli")
                                            .args(["connection", "down", other])
                                            .status();
                                    }
                                }
                                // Activate the selected WireGuard profile via NetworkManager
                                let _ = Command::new("nmcli")
                                    .args(["connection", "up", &loc])
                                    .status();
                            } else {
                                // Disconnect all 3 WireGuard profiles
                                for item in ["Delhi", "London", "Chicago"] {
                                    let _ = Command::new("nmcli")
                                        .args(["connection", "down", item])
                                        .status();
                                }
                            }
                        })
                        .await;
                    },
                    |_| Message::UpdateConfig(Config::default()),
                )
                .map(Into::into);
            }
            Message::SelectSurfsharkLocation(loc) => {
                self.surfshark_location = loc.clone();
                let is_on = self.surfshark_on;
                return Task::perform(
                    async move {
                        let _ = tokio::task::spawn_blocking(move || {
                            if is_on {
                                // Deactivate any other active vpn profiles first
                                for other in ["Delhi", "London", "Chicago"] {
                                    if !other.eq_ignore_ascii_case(&loc) {
                                        let _ = Command::new("nmcli")
                                            .args(["connection", "down", other])
                                            .status();
                                    }
                                }
                                // Connect to the new selected profile
                                let _ = Command::new("nmcli")
                                    .args(["connection", "up", &loc])
                                    .status();
                            }
                        })
                        .await;
                    },
                    |_| Message::UpdateConfig(Config::default()),
                )
                .map(Into::into);
            }

            // Owly AirDrop Handler
            Message::ToggleOwly => {
                self.owly_on = !self.owly_on;
                let enable = self.owly_on;
                return Task::perform(
                    async move {
                        let _ = tokio::task::spawn_blocking(move || {
                            if enable {
                                Command::new("owly").args(["radio", "up"]).status()
                            } else {
                                Command::new("owly").args(["radio", "down"]).status()
                            }
                        })
                        .await;
                    },
                    |_| Message::UpdateConfig(Config::default()),
                )
                .map(Into::into);
            }

            // Audio Effects Handlers (Uses -w / --hide-window to prevent launching GUI)
            Message::ToggleDolbyAtmos => {
                self.dolby_atmos_on = !self.dolby_atmos_on;
                let enable = self.dolby_atmos_on;
                return Task::perform(
                    async move {
                        let _ = tokio::task::spawn_blocking(move || {
                            if enable {
                                Command::new("easyeffects")
                                    .args(["-w", "-l", "DolbyAtmos"])
                                    .status()
                            } else {
                                Command::new("easyeffects")
                                    .args(["-w", "-p", "Bypass"])
                                    .status()
                            }
                        })
                        .await;
                    },
                    |_| Message::UpdateConfig(Config::default()),
                )
                .map(Into::into);
            }
            Message::ToggleMicNoise => {
                self.mic_noise_on = !self.mic_noise_on;
                let enable = self.mic_noise_on;
                return Task::perform(
                    async move {
                        let _ = tokio::task::spawn_blocking(move || {
                            if enable {
                                Command::new("easyeffects")
                                    .args(["-w", "-l", "DeepFilterNoise"])
                                    .status()
                            } else {
                                Command::new("easyeffects")
                                    .args(["-w", "-p", "BypassMic"])
                                    .status()
                            }
                        })
                        .await;
                    },
                    |_| Message::UpdateConfig(Config::default()),
                )
                .map(Into::into);
            }
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}
