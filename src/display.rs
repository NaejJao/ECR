//! Standalone second-screen order display running in its own process.
//!
//! The cashier window publishes a small JSON snapshot and starts this process
//! when the monitor button is enabled. Keeping the display in a separate
//! process isolates its event loop, rendering, and window from the cashier UI,
//! which is what makes the main window stay responsive on Wayland.

use crate::{domain::money, model::Item, ui::widgets::page_frame};
use eframe::egui;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// How often the cashier process refreshes its liveness file.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(2);
/// Slice used to stop the heartbeat thread promptly.
const HEARTBEAT_SLICE: Duration = Duration::from_millis(100);
/// How long the display tolerates a missing heartbeat before closing itself.
const HEARTBEAT_TIMEOUT_MS: u64 = 6000;
/// How often the display checks the snapshot and heartbeat files.
const WATCH_INTERVAL: Duration = Duration::from_millis(400);

/// Snapshot of the current order rendered by the second screen.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct OrderDisplay {
    /// Lines in the same display order and numbering as the sale panel.
    pub(crate) lines: Vec<OrderDisplayLine>,
    /// Total amount due in centimes.
    pub(crate) total: u64,
}

/// One read-only order line shown on the second screen.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct OrderDisplayLine {
    /// Stable index of the line in the open order, used for egui identifiers.
    pub(crate) order_index: usize,
    /// Base product with its nested toppings.
    pub(crate) item: Item,
    /// Per-category number shown after the name when the category has toppings.
    pub(crate) ordinal: Option<usize>,
}

/// Liveness marker written by the cashier process while the display is open.
#[derive(Clone, Copy, Deserialize, Serialize)]
struct Heartbeat {
    /// Identifier of the cashier session that started the display.
    session: u64,
    /// Wall-clock time in Unix milliseconds of the latest heartbeat.
    time_ms: u64,
}

/// Last known window geometry, restored when the display is opened again.
#[derive(Clone, Copy, PartialEq, Deserialize, Serialize)]
struct WindowGeometry {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

/// Child process showing the current order on the second screen.
pub(crate) struct DisplaySession {
    child: Child,
    snapshot_path: PathBuf,
    heartbeat_path: PathBuf,
    heartbeat_stop: Arc<AtomicBool>,
    heartbeat_thread: Option<JoinHandle<()>>,
    last_display: OrderDisplay,
}

impl DisplaySession {
    /// Publishes `display` and starts the separate display process.
    pub(crate) fn start(
        snapshot_path: PathBuf,
        heartbeat_path: PathBuf,
        display: OrderDisplay,
    ) -> Result<Self, String> {
        write_json_atomic(&snapshot_path, &display).map_err(|error| {
            format!("Impossible d’écrire la commande du second écran : {error}")
        })?;
        let session = session_id();
        write_heartbeat(&heartbeat_path, session)
            .map_err(|error| format!("Impossible de préparer le second écran : {error}"))?;

        let executable = std::env::current_exe()
            .map_err(|error| format!("Impossible de localiser l’application : {error}"))?;
        let mut command = Command::new(executable);
        command
            .arg("--order-display")
            .arg(&snapshot_path)
            .arg(&heartbeat_path)
            .stdin(Stdio::null());
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        let child = command
            .spawn()
            .map_err(|error| format!("Impossible d’ouvrir le second écran : {error}"))?;

        let heartbeat_stop = Arc::new(AtomicBool::new(false));
        let heartbeat_thread = {
            let stop = Arc::clone(&heartbeat_stop);
            let path = heartbeat_path.clone();
            std::thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    let _ = write_heartbeat(&path, session);
                    for _ in 0..(HEARTBEAT_INTERVAL.as_millis() / HEARTBEAT_SLICE.as_millis()) {
                        if stop.load(Ordering::Relaxed) {
                            return;
                        }
                        std::thread::sleep(HEARTBEAT_SLICE);
                    }
                }
            })
        };

        Ok(Self {
            child,
            snapshot_path,
            heartbeat_path,
            heartbeat_stop,
            heartbeat_thread: Some(heartbeat_thread),
            last_display: display,
        })
    }

    /// Publishes `display` when it changed; returns `false` once the child exited.
    pub(crate) fn sync(&mut self, display: OrderDisplay) -> bool {
        match self.child.try_wait() {
            Ok(Some(_)) | Err(_) => return false,
            Ok(None) => {}
        }
        if display != self.last_display && write_json_atomic(&self.snapshot_path, &display).is_ok()
        {
            self.last_display = display;
        }
        true
    }
}

impl Drop for DisplaySession {
    fn drop(&mut self) {
        self.heartbeat_stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.heartbeat_thread.take() {
            let _ = thread.join();
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_file(&self.heartbeat_path);
    }
}

/// Runs the display-only application in the child process.
pub(crate) fn run(snapshot_path: PathBuf, heartbeat_path: PathBuf) -> eframe::Result {
    let session = read_json::<Heartbeat>(&heartbeat_path).map_or(0, |beat| beat.session);
    let display = read_json::<OrderDisplay>(&snapshot_path).unwrap_or_default();
    let geometry_path = snapshot_path.with_extension("win");
    let geometry = read_json::<WindowGeometry>(&geometry_path);

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("ECR — Commande en cours")
        .with_min_inner_size([720.0, 480.0])
        .with_active(false);
    viewport = match geometry {
        Some(geometry) if geometry.width >= 720.0 && geometry.height >= 480.0 => viewport
            .with_position([geometry.x, geometry.y])
            .with_inner_size([geometry.width, geometry.height]),
        _ => viewport
            .with_inner_size([1280.0, 800.0])
            .with_maximized(true),
    };

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "ECR — Commande en cours",
        options,
        Box::new(move |creation| {
            crate::ui::apply_style(&creation.egui_ctx);
            spawn_watcher(
                creation.egui_ctx.clone(),
                snapshot_path.clone(),
                heartbeat_path.clone(),
                session,
            );
            Ok(Box::new(OrderDisplayApp {
                snapshot_path,
                geometry_path,
                display,
                last_geometry: None,
                last_geometry_saved: Instant::now(),
            }))
        }),
    )
}

/// Minimal read-only application rendered in the separate display window.
struct OrderDisplayApp {
    snapshot_path: PathBuf,
    geometry_path: PathBuf,
    display: OrderDisplay,
    last_geometry: Option<WindowGeometry>,
    last_geometry_saved: Instant,
}

impl eframe::App for OrderDisplayApp {
    /// Reloads the published order and keeps the window geometry up to date.
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if let Some(display) = read_json::<OrderDisplay>(&self.snapshot_path) {
            self.display = display;
        }
        egui::CentralPanel::default()
            .frame(page_frame())
            .show(ctx, |ui| order_display_ui(ui, &self.display));
        self.save_geometry(ctx);
    }
}

impl OrderDisplayApp {
    /// Remembers the window rectangle so the display reopens on the same screen.
    fn save_geometry(&mut self, ctx: &egui::Context) {
        let (inner, outer) =
            ctx.input(|input| (input.viewport().inner_rect, input.viewport().outer_rect));
        let (Some(inner), Some(outer)) = (inner, outer) else {
            return;
        };
        let geometry = WindowGeometry {
            x: outer.min.x,
            y: outer.min.y,
            width: inner.width(),
            height: inner.height(),
        };
        let changed = self.last_geometry.is_none_or(|last| {
            (last.x - geometry.x).abs() > 4.0
                || (last.y - geometry.y).abs() > 4.0
                || (last.width - geometry.width).abs() > 4.0
                || (last.height - geometry.height).abs() > 4.0
        });
        if changed && self.last_geometry_saved.elapsed() >= Duration::from_secs(1) {
            if write_json_atomic(&self.geometry_path, &geometry).is_ok() {
                self.last_geometry = Some(geometry);
            }
            self.last_geometry_saved = Instant::now();
        }
    }
}

/// Watches the snapshot and heartbeat files from a background thread.
fn spawn_watcher(
    ctx: egui::Context,
    snapshot_path: PathBuf,
    heartbeat_path: PathBuf,
    session: u64,
) {
    std::thread::spawn(move || {
        let mut last_stamp = file_stamp(&snapshot_path);
        loop {
            std::thread::sleep(WATCH_INTERVAL);
            let alive = read_json::<Heartbeat>(&heartbeat_path).is_some_and(|beat| {
                beat.session == session
                    && now_ms().saturating_sub(beat.time_ms) <= HEARTBEAT_TIMEOUT_MS
            });
            if !alive {
                ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
                return;
            }
            let stamp = file_stamp(&snapshot_path);
            if stamp != last_stamp {
                last_stamp = stamp;
                ctx.request_repaint();
            }
        }
    });
}

/// Renders the read-only order content split into two equal-width columns.
fn order_display_ui(ui: &mut egui::Ui, display: &OrderDisplay) {
    ui.horizontal(|ui| {
        ui.heading(egui::RichText::new("Commande en cours").size(42.0).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(format!("À PAYER  {}", money(display.total)))
                    .size(44.0)
                    .strong(),
            );
        });
    });
    ui.separator();
    if display.lines.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new("Aucune commande en cours.").size(34.0));
        });
        return;
    }

    let split = split_for_two_columns(&display.lines);
    let (left, right) = display.lines.split_at(split);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.columns(2, |columns| {
                for (column, lines) in columns.iter_mut().zip([left, right]) {
                    for line in lines {
                        order_display_card(column, line);
                    }
                }
            });
        });
}

/// Splits order lines into two columns with roughly equal estimated heights.
fn split_for_two_columns(entries: &[OrderDisplayLine]) -> usize {
    let item_height = |line: &OrderDisplayLine| 52.0 + line.item.toppings.len() as f32 * 32.0;
    let total: f32 = entries.iter().map(item_height).sum();
    let half = total / 2.0;
    let mut accumulated = 0.0;
    let mut split = 0;
    for (position, line) in entries.iter().enumerate() {
        let height = item_height(line);
        if position > 0 && accumulated + height > half {
            split = position;
            break;
        }
        accumulated += height;
        split = position + 1;
    }
    split
}

/// Draws one read-only order card in the second-screen view.
fn order_display_card(ui: &mut egui::Ui, line: &OrderDisplayLine) {
    let item = &line.item;
    let tint = egui::Color32::from_rgb(item.theme[0], item.theme[1], item.theme[2]);
    egui::Frame::new()
        .fill(tint)
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(16, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let item_name = if item.quantity > 1 {
                    format!("{} x {}", item.quantity, item.name)
                } else {
                    item.name.clone()
                };
                let title = match line.ordinal {
                    Some(ordinal) => format!("{} {}", item_name, ordinal),
                    None => item_name,
                };
                ui.label(egui::RichText::new(title).size(23.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(money(
                            item.price.saturating_mul(u64::from(item.quantity)),
                        ))
                        .size(21.0)
                        .strong(),
                    );
                });
            });
            ui.indent(("order-display-toppings", line.order_index), |ui| {
                for topping in &item.toppings {
                    ui.horizontal(|ui| {
                        let name = if topping.quantity > 1 {
                            format!("Garniture : {} x {}", topping.quantity, topping.name)
                        } else {
                            format!("Garniture : {}", topping.name)
                        };
                        ui.label(egui::RichText::new(name).size(19.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(money(
                                    topping.price.saturating_mul(u64::from(topping.quantity)),
                                ))
                                .size(18.0),
                            );
                        });
                    });
                }
            });
        });
    ui.add_space(8.0);
}

/// Unix time in milliseconds.
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as u64)
}

/// Identifier unique to this cashier session and process.
fn session_id() -> u64 {
    (now_ms() << 20) ^ u64::from(std::process::id())
}

/// (Last modification time, file length) pair used for change detection.
fn file_stamp(path: &Path) -> Option<(SystemTime, u64)> {
    let metadata = fs::metadata(path).ok()?;
    Some((metadata.modified().ok()?, metadata.len()))
}

/// Reads and parses a JSON file, returning `None` when it is missing or invalid.
fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

/// Serializes a value and replaces the destination file atomically.
fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut temporary = path.as_os_str().to_os_string();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);
    fs::write(&temporary, bytes)?;
    fs::rename(temporary, path)
}

/// Writes the liveness marker read by the display process.
fn write_heartbeat(path: &Path, session: u64) -> std::io::Result<()> {
    write_json_atomic(
        path,
        &Heartbeat {
            session,
            time_ms: now_ms(),
        },
    )
}
