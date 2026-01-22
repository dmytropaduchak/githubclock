use chrono::{Datelike, Local, Timelike};
use macroquad::prelude::*;
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Tree};
use std::cell::RefCell;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use miniquad::conf::Conf;
use miniquad::conf::Icon;
use std::error::Error;
use std::fs;

fn icon<const SIZE: usize>(path: &str) -> Result<[u8; SIZE], Box<dyn Error>> {
    let data = fs::read(path).map_err(|e| format!("Failed to read {}: {}", path, e))?;
    let len = data.len();
    data.try_into().map_err(|_| {
        format!(
            "{} has invalid size (expected {} bytes, got {})",
            path, SIZE, len
        )
        .into()
    })
}

fn round_icon_corners<const SIZE: usize>(data: &mut [u8; SIZE], dim: u32) {
    let r = (dim as f32 * 0.22).round() as u32;
    let r2 = r * r;
    for y in 0..dim {
        for x in 0..dim {
            let in_corner = (x < r || x >= dim - r) && (y < r || y >= dim - r);
            if in_corner {
                let cx = if x < r { r } else { dim - 1 - r };
                let cy = if y < r { r } else { dim - 1 - r };
                let dx = x.abs_diff(cx);
                let dy = y.abs_diff(cy);
                if dx * dx + dy * dy > r2 {
                    let idx = ((y * dim + x) * 4 + 3) as usize;
                    data[idx] = 0;
                }
            }
        }
    }
}

fn files() -> Result<Icon, Box<dyn Error>> {
    let mut small = icon::<{ 16 * 16 * 4 }>("icon_16.rgba")?;
    let mut medium = icon::<{ 32 * 32 * 4 }>("icon_32.rgba")?;
    let mut big = icon::<{ 64 * 64 * 4 }>("icon_64.rgba")?;
    round_icon_corners(&mut small, 16);
    round_icon_corners(&mut medium, 32);
    round_icon_corners(&mut big, 64);
    Ok(Icon { small, medium, big })
}

pub fn conf() -> Conf {
    let icon = match files() {
        Ok(icon) => Some(icon),
        Err(e) => {
            eprintln!("Failed to load icons: {e}");
            None
        }
    };

    Conf {
        window_title: "".to_string(),
        window_width: 640,
        window_height: 260,
        window_resizable: false,
        icon,
        ..Default::default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HourFormat {
    H24,
    H12,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TimeFormat {
    HhMmSs,
    HhMm,
    MmSs,
    IsoTime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConnectionStatus {
    Unknown,
    Connected,
    Disconnected,
}

#[derive(Clone, Debug)]
struct GithubPr {
    title: String,
    url: String,
}

#[derive(Clone, Debug)]
struct GithubFetchResult {
    connected: bool,
    prs: Vec<GithubPr>,
}

#[derive(Clone, Copy, Debug)]
struct ClockLayout {
    time_bottom: f32,
    left_x: f32,
    board_grid: PixelGrid,
    pr_grid: PixelGrid,
}

#[derive(Clone, Debug)]
struct PrHit {
    rect: Rect,
    url: String,
}

#[derive(Clone, Copy, Debug)]
struct Theme {
    background_color: Color,
    inactive_color: Color,
    active_color: Color,
    noise_color: Color,
    active_alpha: f32,
    active_alpha_jitter: f32,
}

#[derive(Clone, Copy, Debug)]
struct PixelGrid {
    cell: f32,
    gap: f32,
}

impl PixelGrid {
    fn step(self) -> f32 {
        self.cell + self.gap
    }
}

#[derive(Clone, Copy, Debug)]
struct FrameContext {
    theme: Theme,
    container: Rect,
}

impl Default for FrameContext {
    fn default() -> Self {
        let theme = Theme {
            background_color: Color {
                r: 0.06,
                g: 0.07,
                b: 0.08,
                a: 1.0,
            },
            inactive_color: Color {
                r: 0.12,
                g: 0.13,
                b: 0.15,
                a: 1.0,
            },
            active_color: Color {
                r: 0.2,
                g: 0.85,
                b: 0.82,
                a: 1.0,
            },
            noise_color: Color {
                r: 0.2,
                g: 0.85,
                b: 0.82,
                a: 1.0,
            },
            active_alpha: 0.82,
            active_alpha_jitter: 0.4,
        };

        FrameContext {
            theme,
            container: Rect::new(20.0, 20.0, 440.0, 220.0),
        }
    }
}

thread_local! {
    static FRAME_CONTEXT: RefCell<FrameContext> = RefCell::new(FrameContext::default());
}

const GITHUB_ICON_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 640"><path fill="#ffffff" d="M237.9 461.4C237.9 463.4 235.6 465 232.7 465C229.4 465.3 227.1 463.7 227.1 461.4C227.1 459.4 229.4 457.8 232.3 457.8C235.3 457.5 237.9 459.1 237.9 461.4zM206.8 456.9C206.1 458.9 208.1 461.2 211.1 461.8C213.7 462.8 216.7 461.8 217.3 459.8C217.9 457.8 216 455.5 213 454.6C210.4 453.9 207.5 454.9 206.8 456.9zM251 455.2C248.1 455.9 246.1 457.8 246.4 460.1C246.7 462.1 249.3 463.4 252.3 462.7C255.2 462 257.2 460.1 256.9 458.1C256.6 456.2 253.9 454.9 251 455.2zM316.8 72C178.1 72 72 177.3 72 316C72 426.9 141.8 521.8 241.5 555.2C254.3 557.5 258.8 549.6 258.8 543.1C258.8 536.9 258.5 502.7 258.5 481.7C258.5 481.7 188.5 496.7 173.8 451.9C173.8 451.9 162.4 422.8 146 415.3C146 415.3 123.1 399.6 147.6 399.9C147.6 399.9 172.5 401.9 186.2 425.7C208.1 464.3 244.8 453.2 259.1 446.6C261.4 430.6 267.9 419.5 275.1 412.9C219.2 406.7 162.8 398.6 162.8 302.4C162.8 274.9 170.4 261.1 186.4 243.5C183.8 237 175.3 210.2 189 175.6C209.9 169.1 258 202.6 258 202.6C278 197 299.5 194.1 320.8 194.1C342.1 194.1 363.6 197 383.6 202.6C383.6 202.6 431.7 169 452.6 175.6C466.3 210.3 457.8 237 455.2 243.5C471.2 261.2 481 275 481 302.4C481 398.9 422.1 406.6 366.2 412.9C375.4 420.8 383.2 435.8 383.2 459.3C383.2 493 382.9 534.7 382.9 542.9C382.9 549.4 387.5 557.3 400.2 555C500.2 521.8 568 426.9 568 316C568 177.3 455.5 72 316.8 72zM169.2 416.9C167.9 417.9 168.2 420.2 169.9 422.1C171.5 423.7 173.8 424.4 175.1 423.1C176.4 422.1 176.1 419.8 174.4 417.9C172.8 416.3 170.5 415.6 169.2 416.9zM158.4 408.8C157.7 410.1 158.7 411.7 160.7 412.7C162.3 413.7 164.3 413.4 165 412C165.7 410.7 164.7 409.1 162.7 408.1C160.7 407.5 159.1 407.8 158.4 408.8zM190.8 444.4C189.2 445.7 189.8 448.7 192.1 450.6C194.4 452.9 197.3 453.2 198.6 451.6C199.9 450.3 199.3 447.3 197.3 445.4C195.1 443.1 192.1 442.8 190.8 444.4zM179.4 429.7C177.8 430.7 177.8 433.3 179.4 435.6C181 437.9 183.7 438.9 185 437.9C186.6 436.6 186.6 434 185 431.7C183.6 429.4 181 428.4 179.4 429.7z"/></svg>"##;

const PR_ICON_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 640"><path fill="#ffffff" d="M176 120C189.3 120 200 130.7 200 144C200 157.3 189.3 168 176 168C162.7 168 152 157.3 152 144C152 130.7 162.7 120 176 120zM208.4 217.2C236.4 204.8 256 176.7 256 144C256 99.8 220.2 64 176 64C131.8 64 96 99.8 96 144C96 176.8 115.7 205 144 217.3L144 422.6C115.7 435 96 463.2 96 496C96 540.2 131.8 576 176 576C220.2 576 256 540.2 256 496C256 463.2 236.3 435 208 422.7L208 336.1C234.7 356.2 268 368.1 304 368.1L390.7 368.1C403 396.4 431.2 416.1 464 416.1C508.2 416.1 544 380.3 544 336.1C544 291.9 508.2 256.1 464 256.1C431.2 256.1 403 275.8 390.7 304.1L304 304C254.1 304 213 265.9 208.4 217.2zM176 472C189.3 472 200 482.7 200 496C200 509.3 189.3 520 176 520C162.7 520 152 509.3 152 496C152 482.7 162.7 472 176 472zM440 336C440 322.7 450.7 312 464 312C477.3 312 488 322.7 488 336C488 349.3 477.3 360 464 360C450.7 360 440 349.3 440 336z"/></svg>"##;

fn update_context(theme: Theme, container: Rect) {
    FRAME_CONTEXT.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        ctx.theme = theme;
        ctx.container = container;
    });
}

fn load_github_icon_texture(size: u32) -> Option<Texture2D> {
    let opt = Options::default();
    let tree = Tree::from_str(GITHUB_ICON_SVG, &opt).ok()?;
    let mut pixmap = Pixmap::new(size, size)?;
    let svg_size = tree.size();
    let scale = (size as f32 / svg_size.width()).min(size as f32 / svg_size.height());
    let tx = (size as f32 - svg_size.width() * scale) * 0.5;
    let ty = (size as f32 - svg_size.height() * scale) * 0.5;
    let transform = Transform::from_scale(scale, scale).post_translate(tx, ty);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let texture = Texture2D::from_rgba8(size as u16, size as u16, pixmap.data());
    texture.set_filter(FilterMode::Nearest);
    Some(texture)
}

fn load_pr_icon_texture(size: u32) -> Option<Texture2D> {
    let opt = Options::default();
    let tree = Tree::from_str(PR_ICON_SVG, &opt).ok()?;
    let mut pixmap = Pixmap::new(size, size)?;
    let svg_size = tree.size();
    let scale = (size as f32 / svg_size.width()).min(size as f32 / svg_size.height());
    let tx = (size as f32 - svg_size.width() * scale) * 0.5;
    let ty = (size as f32 - svg_size.height() * scale) * 0.5;
    let transform = Transform::from_scale(scale, scale).post_translate(tx, ty);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let texture = Texture2D::from_rgba8(size as u16, size as u16, pixmap.data());
    texture.set_filter(FilterMode::Nearest);
    Some(texture)
}

fn spawn_github_fetch(token: String) -> mpsc::Receiver<GithubFetchResult> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(4))
            .build();
        let auth_header = format!("Bearer {}", token);
        let user_resp = agent
            .get("https://api.github.com/user")
            .set("User-Agent", "commit-clock")
            .set("Authorization", &auth_header)
            .set("Accept", "application/vnd.github+json")
            .call();

        let user_resp = match user_resp {
            Ok(resp) if (200..300).contains(&resp.status()) => resp,
            _ => {
                let _ = tx.send(GithubFetchResult {
                    connected: false,
                    prs: Vec::new(),
                });
                return;
            }
        };

        let user_status = user_resp.status();
        let user_json: serde_json::Value = match user_resp.into_string() {
            Ok(body) => {
                println!("GitHub user status: {}", user_status);
                println!("GitHub user response: {}", body);
                serde_json::from_str(&body).unwrap_or(serde_json::Value::Null)
            }
            Err(_) => {
                let _ = tx.send(GithubFetchResult {
                    connected: false,
                    prs: Vec::new(),
                });
                return;
            }
        };

        let login = match user_json.get("login").and_then(|value| value.as_str()) {
            Some(login) => login.to_string(),
            None => {
                let _ = tx.send(GithubFetchResult {
                    connected: false,
                    prs: Vec::new(),
                });
                return;
            }
        };

        let query = format!(
            "https://api.github.com/search/issues?q=is:pr+is:open+author:{}&per_page=3&sort=updated&order=desc",
            login
        );
        println!("GitHub PR query: {}", query);
        let prs_resp = agent
            .get(&query)
            .set("User-Agent", "commit-clock")
            .set("Authorization", &auth_header)
            .set("Accept", "application/vnd.github+json")
            .call();

        let prs_resp = match prs_resp {
            Ok(resp) if (200..300).contains(&resp.status()) => resp,
            _ => {
                let _ = tx.send(GithubFetchResult {
                    connected: true,
                    prs: Vec::new(),
                });
                return;
            }
        };

        let prs_status = prs_resp.status();
        let prs_json: serde_json::Value = match prs_resp.into_string() {
            Ok(body) => {
                println!("GitHub PR status: {}", prs_status);
                println!("GitHub PR response: {}", body);
                serde_json::from_str(&body).unwrap_or(serde_json::Value::Null)
            }
            Err(_) => {
                let _ = tx.send(GithubFetchResult {
                    connected: true,
                    prs: Vec::new(),
                });
                return;
            }
        };

        let mut prs = prs_json
            .get("items")
            .and_then(|items| items.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        let title = item.get("title").and_then(|t| t.as_str())?;
                        let url = item.get("html_url").and_then(|u| u.as_str())?;
                        Some(GithubPr {
                            title: title.to_string(),
                            url: url.to_string(),
                        })
                    })
                    .take(3)
                    .collect::<Vec<GithubPr>>()
            })
            .unwrap_or_default();

        if prs.is_empty() {
            let repos_url = "https://api.github.com/user/repos?affiliation=owner,collaborator,organization_member&per_page=50&sort=updated";
            println!("GitHub repos query: {}", repos_url);
            let repos_resp = agent
                .get(repos_url)
                .set("User-Agent", "commit-clock")
                .set("Authorization", &auth_header)
                .set("Accept", "application/vnd.github+json")
                .call();

            let repos_resp = match repos_resp {
                Ok(resp) if (200..300).contains(&resp.status()) => resp,
                _ => {
                    let _ = tx.send(GithubFetchResult {
                        connected: true,
                        prs: Vec::new(),
                    });
                    return;
                }
            };

            let repos_status = repos_resp.status();
            let repos_json: serde_json::Value = match repos_resp.into_string() {
                Ok(body) => {
                    println!("GitHub repos status: {}", repos_status);
                    println!("GitHub repos response: {}", body);
                    serde_json::from_str(&body).unwrap_or(serde_json::Value::Null)
                }
                Err(_) => serde_json::Value::Null,
            };

            let repos = repos_json
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.get("full_name").and_then(|v| v.as_str()))
                        .take(20)
                        .map(|name| name.to_string())
                        .collect::<Vec<String>>()
                })
                .unwrap_or_default();

            let mut matches: Vec<(String, GithubPr)> = Vec::new();
            for repo in repos {
                let pulls_url = format!(
                    "https://api.github.com/repos/{}/pulls?state=open&per_page=10&sort=updated&direction=desc",
                    repo
                );
                let pulls_resp = agent
                    .get(&pulls_url)
                    .set("User-Agent", "commit-clock")
                    .set("Authorization", &auth_header)
                    .set("Accept", "application/vnd.github+json")
                    .call();

                let pulls_resp = match pulls_resp {
                    Ok(resp) if (200..300).contains(&resp.status()) => resp,
                    _ => continue,
                };

                let pulls_json: serde_json::Value = match pulls_resp.into_string() {
                    Ok(body) => {
                        serde_json::from_str(&body).unwrap_or(serde_json::Value::Null)
                    }
                    Err(_) => serde_json::Value::Null,
                };

                let pulls = match pulls_json.as_array() {
                    Some(items) => items,
                    None => continue,
                };

                for pr in pulls {
                    let author = pr
                        .get("user")
                        .and_then(|u| u.get("login"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let title = pr.get("title").and_then(|v| v.as_str()).unwrap_or("");
                    let url = pr.get("html_url").and_then(|v| v.as_str()).unwrap_or("");
                    let updated = pr.get("updated_at").and_then(|v| v.as_str()).unwrap_or("");

                    if author == login {
                        matches.push((
                            updated.to_string(),
                            GithubPr {
                                title: title.to_string(),
                                url: url.to_string(),
                            },
                        ));
                    }
                }
            }

            matches.sort_by(|a, b| b.0.cmp(&a.0));
            prs = matches.into_iter().map(|(_, pr)| pr).take(3).collect();
        }

        let _ = tx.send(GithubFetchResult {
            connected: true,
            prs,
        });
    });
    rx
}

fn load_github_token() -> Option<String> {
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        let token = token.trim().to_string();
        if !token.is_empty() {
            return Some(token);
        }
    }

    if let Ok(token) = std::env::var("CHRONO_GITHUB_TOKEN") {
        let token = token.trim().to_string();
        if !token.is_empty() {
            return Some(token);
        }
    }

    let home = std::env::var("HOME").ok()?;

    let config_path = format!("{}/.config/.githubclock", home);
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        for line in content.lines() {
            if let Some(value) = line.strip_prefix("GITHUB_TOKEN=") {
                let token = value.trim().to_string();
                if !token.is_empty() {
                    return Some(token);
                }
            }
        }
    }

    None
}

fn ensure_config_file() {
    let home = match std::env::var("HOME") {
        Ok(h) => h,
        _ => return,
    };
    let config_path = format!("{}/.config/.githubclock", home);
    if std::path::Path::new(&config_path).exists() {
        return;
    }
    if std::env::var("GITHUB_TOKEN").is_ok() || std::env::var("CHRONO_GITHUB_TOKEN").is_ok() {
        return;
    }
    let _ = std::fs::write(&config_path, "GITHUB_TOKEN=\n");
    open_url(&config_path);
}

fn format_time(hour_format: HourFormat, time_format: TimeFormat) -> String {
    let now = Local::now();
    let mut hour = now.hour() as i32;
    let minute = now.minute();
    let second = now.second();

    if hour_format == HourFormat::H12 {
        hour %= 12;
        if hour == 0 {
            hour = 12;
        }
    }

    match time_format {
        TimeFormat::HhMmSs => format!("{:02}:{:02}:{:02}", hour, minute, second),
        TimeFormat::HhMm => format!("{:02}:{:02}", hour, minute),
        TimeFormat::MmSs => format!("{:02}:{:02}", minute, second),
        TimeFormat::IsoTime => format!("{:02}:{:02}:{:02}", hour, minute, second),
    }
}

fn am_pm_suffix(hour_format: HourFormat) -> Option<String> {
    if hour_format == HourFormat::H24 {
        return None;
    }
    let hour = Local::now().hour();
    if hour >= 12 {
        Some("PM".to_string())
    } else {
        Some("AM".to_string())
    }
}

fn format_year() -> String {
    Local::now().year().to_string()
}

fn format_day_month() -> String {
    let now = Local::now();
    let day = now.day();
    let month_name = match now.month() {
        1 => "JAN",
        2 => "FEB",
        3 => "MAR",
        4 => "APR",
        5 => "MAY",
        6 => "JUN",
        7 => "JUL",
        8 => "AUG",
        9 => "SEP",
        10 => "OCT",
        11 => "NOV",
        _ => "DEC",
    };
    format!("{:02}{}", day, month_name)
}

fn grid_from_height(target_height: f32, gap_ratio: f32) -> PixelGrid {
    let cell = (target_height / 7.0).round().max(1.0);
    let gap = (cell * gap_ratio).round().max(1.0);
    PixelGrid { cell, gap }
}

fn draw_background(board_grid: PixelGrid) {
    FRAME_CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        clear_background(ctx.theme.background_color);
        draw_grid(ctx.container, board_grid, ctx.theme.inactive_color);
    });
}

fn draw_noise_pixels(board_grid: PixelGrid) {
    FRAME_CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        let rect = ctx.container;
        let step = (board_grid.step() * 1.4).round().max(6.0) as i32;
        let dot = (board_grid.cell * 0.35).max(2.0);
        for y in (rect.y as i32..(rect.y + rect.h) as i32).step_by(step as usize) {
            for x in (rect.x as i32..(rect.x + rect.w) as i32).step_by(step as usize) {
                let hash = (x * 37 + y * 101) & 255;
                if hash < 22 {
                    let alpha = 0.03 + (hash as f32 / 255.0) * 0.04;
                    draw_rectangle(
                        x as f32 + 2.0,
                        y as f32 + 2.0,
                        dot,
                        dot,
                        Color::new(
                            ctx.theme.noise_color.r,
                            ctx.theme.noise_color.g,
                            ctx.theme.noise_color.b,
                            alpha,
                        ),
                    );
                }
            }
        }
    });
}

fn draw_active_speckles(board_grid: PixelGrid, minute_seed: i32, blocked: &[Rect]) {
    FRAME_CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        let rect = ctx.container;
        let step = board_grid.step();
        let cols = (rect.w / step).ceil() as i32;
        let rows = (rect.h / step).ceil() as i32;
        let total = (cols * rows).max(1);
        let mut picks: Vec<(i32, i32)> = Vec::new();
        for i in 0..9 {
            let mut idx = ((minute_seed * 997 + i * 379) % total).abs();
            for _ in 0..total {
                let row = idx / cols;
                let col = idx % cols;
                let parity = (row + col) & 1;
                if parity == 0 && !picks.contains(&(row, col)) {
                    picks.push((row, col));
                    break;
                }
                idx = (idx + 1) % total;
            }
        }

        for (i, (row, col)) in picks.iter().enumerate() {
            let alpha = if i < 3 {
                0.35 + ((row * 13 + col * 7) & 15) as f32 / 120.0
            } else if i < 6 {
                0.65 + ((row * 23 + col * 11) & 15) as f32 / 120.0
            } else {
                0.95 + ((row * 31 + col * 17) & 7) as f32 / 100.0
            };
            let speck_rect = Rect::new(
                rect.x + *col as f32 * step,
                rect.y + *row as f32 * step,
                board_grid.cell,
                board_grid.cell,
            );
            if rect_overlaps_any(speck_rect, blocked) {
                continue;
            }
            draw_rectangle(
                speck_rect.x,
                speck_rect.y,
                speck_rect.w,
                speck_rect.h,
                Color::new(
                    ctx.theme.active_color.r,
                    ctx.theme.active_color.g,
                    ctx.theme.active_color.b,
                    alpha.min(1.0),
                ),
            );
        }
    });
}

fn draw_grid(rect: Rect, grid: PixelGrid, color: Color) {
    let step = grid.step();
    let cols = (rect.w / step).ceil() as i32;
    let rows = (rect.h / step).ceil() as i32;
    for row in 0..rows {
        for col in 0..cols {
            let x = rect.x + col as f32 * step;
            let y = rect.y + row as f32 * step;
            draw_rectangle(x, y, grid.cell, grid.cell, color);
        }
    }
}

fn draw_pixel_text(text: &str, origin: Vec2, grid: PixelGrid, color: Color, cutout: bool) {
    let step = grid.step();
    let spacing = glyph_spacing(grid);
    let mut cursor_x = origin.x;
    for ch in text.chars() {
        // 5x7 glyphs with pixel-based inter-character spacing.
        let glyph = glyph_pattern(ch);
        if let Some((min_x, max_x)) = glyph_bounds(glyph) {
            let width_cols = (max_x - min_x + 1) as f32;
            for (row, line) in glyph.iter().enumerate() {
                for (col, cell) in line.chars().enumerate() {
                    if cell == '#' {
                        let x = cursor_x + (col as f32 - min_x as f32) * step;
                        let y = origin.y + row as f32 * step;
                        let draw_color = if cutout {
                            color
                        } else {
                            apply_active_alpha(color, x, y)
                        };
                        draw_rectangle(x, y, grid.cell, grid.cell, draw_color);
                    }
                }
            }
            cursor_x += width_cols * step + spacing;
        } else {
            cursor_x += space_width_cols() * step + spacing;
        }
    }
}

fn measure_pixel_text(text: &str, grid: PixelGrid) -> Vec2 {
    let step = grid.step();
    let spacing = glyph_spacing(grid);
    let mut width = 0.0;
    let mut count = 0usize;
    for ch in text.chars() {
        let glyph = glyph_pattern(ch);
        let cols = if let Some((min_x, max_x)) = glyph_bounds(glyph) {
            (max_x - min_x + 1) as f32
        } else {
            space_width_cols()
        };
        width += cols * step + spacing;
        count += 1;
    }
    if count > 0 {
        width -= spacing;
    }
    let height = step * 7.0 - grid.gap;
    vec2(width, height)
}

fn snap_to_grid(origin: f32, value: f32, step: f32) -> f32 {
    let offset = value - origin;
    origin + (offset / step).round() * step
}

fn rect_overlaps_any(target: Rect, blocked: &[Rect]) -> bool {
    blocked.iter().any(|rect| rects_intersect(target, *rect))
}

fn rects_intersect(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y
}

fn point_in_rect(point: Vec2, rect: Rect) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.w
        && point.y >= rect.y
        && point.y <= rect.y + rect.h
}

fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

fn glyph_bounds(glyph: [&'static str; 7]) -> Option<(usize, usize)> {
    let mut min_x = usize::MAX;
    let mut max_x = 0usize;
    let mut found = false;
    for line in glyph.iter() {
        for (idx, cell) in line.chars().enumerate() {
            if cell == '#' {
                min_x = min_x.min(idx);
                max_x = max_x.max(idx);
                found = true;
            }
        }
    }
    if found {
        Some((min_x, max_x))
    } else {
        None
    }
}

fn glyph_spacing(grid: PixelGrid) -> f32 {
    grid.step()
}

fn space_width_cols() -> f32 {
    3.0
}

fn apply_active_alpha(color: Color, x: f32, y: f32) -> Color {
    FRAME_CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        let hash = ((x as i32 * 29 + y as i32 * 91) & 255) as f32 / 255.0;
        let jitter = (hash - 0.5) * 2.0 * ctx.theme.active_alpha_jitter;
        let alpha = (ctx.theme.active_alpha + jitter).clamp(0.2, 1.0);
        Color::new(color.r, color.g, color.b, alpha)
    })
}

fn draw_clock(
    year_str: &str,
    date_str: &str,
    time_str: &str,
    am_pm: Option<&str>,
    minute_seed: i32,
) -> ClockLayout {
    FRAME_CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        let container = ctx.container;
        let year_grid = grid_from_height(8.0, 0.25);
        let date_grid = grid_from_height(24.0, 0.25);
        let time_grid = grid_from_height(42.0, 0.25);
        let board_grid = time_grid;
        let gap_small = 2.0;
        let gap_large = 2.0;

        let year_size = measure_pixel_text(year_str, year_grid);
        let date_size = measure_pixel_text(date_str, date_grid);
        let time_size = measure_pixel_text(time_str, time_grid);
        let am_pm_size = am_pm
            .map(|value| measure_pixel_text(value, year_grid))
            .unwrap_or(vec2(0.0, 0.0));
        draw_background(board_grid);
        draw_noise_pixels(board_grid);

        let padding = 12.0;
        let mut year_origin = vec2(container.x + padding, container.y + padding);
        year_origin.x = snap_to_grid(container.x, year_origin.x, board_grid.step());
        year_origin.y = snap_to_grid(container.y, year_origin.y, board_grid.step());

        let mut date_origin = vec2(
            container.x + padding,
            year_origin.y + year_size.y + gap_small,
        );
        date_origin.x = snap_to_grid(container.x, date_origin.x, board_grid.step());
        date_origin.y = snap_to_grid(container.y, date_origin.y, board_grid.step());

        let mut time_origin = vec2(
            container.x + padding,
            date_origin.y + date_size.y + gap_large,
        );
        time_origin.x = snap_to_grid(container.x, time_origin.x, board_grid.step());
        time_origin.y = snap_to_grid(container.y, time_origin.y, board_grid.step());

        let active = ctx.theme.active_color;

        let mut blocked = Vec::new();
        blocked.extend(collect_glyph_rects(year_str, year_origin, year_grid));
        blocked.extend(collect_glyph_rects(date_str, date_origin, date_grid));
        blocked.extend(collect_glyph_rects(time_str, time_origin, time_grid));

        let mut am_pm_origin = None;
        if am_pm.is_some() {
            let mut origin = vec2(
                time_origin.x + time_size.x + time_grid.step(),
                time_origin.y + time_size.y - am_pm_size.y,
            );
            origin.x = snap_to_grid(container.x, origin.x, board_grid.step());
            origin.y = snap_to_grid(container.y, origin.y, board_grid.step());
            am_pm_origin = Some(origin);
        }

        if let (Some(suffix), Some(origin)) = (am_pm, am_pm_origin) {
            blocked.extend(collect_glyph_rects(suffix, origin, year_grid));
        }

        draw_active_speckles(board_grid, minute_seed, &blocked);
        draw_pixel_text(year_str, year_origin, year_grid, active, false);
        draw_pixel_text(date_str, date_origin, date_grid, active, false);
        draw_pixel_text(time_str, time_origin, time_grid, active, false);

        if let (Some(suffix), Some(origin)) = (am_pm, am_pm_origin) {
            let am_pm_color = Color::new(active.r, active.g, active.b, 0.75);
            draw_pixel_text(suffix, origin, year_grid, am_pm_color, false);
        }

        ClockLayout {
            time_bottom: time_origin.y + time_size.y,
            left_x: year_origin.x,
            board_grid,
            pr_grid: year_grid,
        }
    })
}

fn github_button_rect(container: Rect, grid: PixelGrid) -> Rect {
    let size = (grid.step() * 3.0).round().max(grid.step());
    let padding = 8.0;
    let mut x = container.x + container.w - size - padding;
    let mut y = container.y + padding;
    x = snap_to_grid(container.x, x, grid.step());
    y = snap_to_grid(container.y, y, grid.step());
    Rect::new(x, y, size, size)
}

fn draw_github_button(status: ConnectionStatus, icon: Option<&Texture2D>, rect: Rect) {
    FRAME_CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        let button_color = Color::new(
            ctx.theme.inactive_color.r * 0.9,
            ctx.theme.inactive_color.g * 0.9,
            ctx.theme.inactive_color.b * 0.9,
            1.0,
        );
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, button_color);

        let icon_color = match status {
            ConnectionStatus::Connected => ctx.theme.active_color,
            ConnectionStatus::Disconnected | ConnectionStatus::Unknown => {
                Color::new(1.0, 1.0, 1.0, 1.0)
            }
        };

        if let Some(texture) = icon {
            let icon_size = rect.w.min(rect.h);
            let icon_x = rect.x + (rect.w - icon_size) * 0.5;
            let icon_y = rect.y + (rect.h - icon_size) * 0.5;
            draw_texture_ex(
                texture,
                icon_x,
                icon_y,
                icon_color,
                DrawTextureParams {
                    dest_size: Some(vec2(icon_size, icon_size)),
                    ..Default::default()
                },
            );
        }
    });
}

fn collect_glyph_rects(text: &str, origin: Vec2, grid: PixelGrid) -> Vec<Rect> {
    let step = grid.step();
    let spacing = glyph_spacing(grid);
    let mut rects = Vec::new();
    let mut cursor_x = origin.x;
    for ch in text.chars() {
        let glyph = glyph_pattern(ch);
        if let Some((min_x, max_x)) = glyph_bounds(glyph) {
            let width_cols = (max_x - min_x + 1) as f32;
            for (row, line) in glyph.iter().enumerate() {
                for (col, cell) in line.chars().enumerate() {
                    if cell == '#' {
                        rects.push(Rect::new(
                            cursor_x + (col as f32 - min_x as f32) * step,
                            origin.y + row as f32 * step,
                            grid.cell,
                            grid.cell,
                        ));
                    }
                }
            }
            cursor_x += width_cols * step + spacing;
        } else {
            cursor_x += space_width_cols() * step + spacing;
        }
    }
    rects
}

fn wrap_text_to_width(text: &str, max_width: f32, font_size: u16) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let spacer = if current.is_empty() { "" } else { " " };
        let candidate = format!("{}{}{}", current, spacer, word);
        let candidate_width = measure_text(&candidate, None, font_size, 1.0).width;

        if candidate_width <= max_width {
            current = candidate;
        } else {
            if !current.is_empty() {
                lines.push(current);
            }
            current = word.to_string();
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn is_jira_key(value: &str) -> bool {
    if let Some((left, right)) = value.split_once('-') {
        if left.len() >= 2
            && !right.is_empty()
            && left.chars().all(|c| c.is_ascii_uppercase())
            && right.chars().all(|c| c.is_ascii_digit())
        {
            return true;
        }
    }
    false
}

fn find_jira_in_line(line: &str) -> Option<(usize, usize, String)> {
    let mut token = String::new();
    let mut token_start = 0usize;

    for (idx, ch) in line.char_indices() {
        if ch.is_ascii_alphanumeric() || ch == '-' {
            if token.is_empty() {
                token_start = idx;
            }
            token.push(ch);
        } else if !token.is_empty() {
            if is_jira_key(&token) {
                return Some((token_start, idx, token.clone()));
            }
            token.clear();
        }
    }

    if !token.is_empty() && is_jira_key(&token) {
        return Some((token_start, line.len(), token));
    }
    None
}

fn draw_pr_list(prs: &[GithubPr], icon: Option<&Texture2D>, layout: ClockLayout) -> Vec<PrHit> {
    FRAME_CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        let step = layout.board_grid.step();
        let offset = step * 3.0;
        let mut y = layout.time_bottom + offset;
        y = snap_to_grid(ctx.container.y, y, layout.board_grid.step());

        let icon_size = 16.0;
        let font_size = 14u16;
        let line_height = font_size as f32 + 4.0;
        let item_gap = 6.0;
        let mut line_y = y;
        let mut hits = Vec::new();
        for pr in prs.iter() {
            let text_x = if icon.is_some() {
                layout.left_x + icon_size + layout.pr_grid.step()
            } else {
                layout.left_x
            };
            let max_width = ctx.container.w - text_x - 12.0;
            let wrapped = wrap_text_to_width(&pr.title, max_width, font_size);
            if wrapped.iter().all(|line| line.trim().is_empty()) {
                continue;
            }
            if let Some(texture) = icon {
                let icon_y = line_y + (line_height - icon_size) * 0.5 + 2.0;
                let (mx, my) = mouse_position();
                let hover = point_in_rect(
                    vec2(mx, my),
                    Rect::new(layout.left_x, icon_y, icon_size, icon_size),
                );
                let icon_color = if hover {
                    Color::new(1.0, 1.0, 1.0, 1.0)
                } else {
                    ctx.theme.active_color
                };
                draw_texture_ex(
                    texture,
                    layout.left_x,
                    icon_y,
                    icon_color,
                    DrawTextureParams {
                        dest_size: Some(vec2(icon_size, icon_size)),
                        ..Default::default()
                    },
                );
                hits.push(PrHit {
                    rect: Rect::new(layout.left_x, icon_y, icon_size, icon_size),
                    url: pr.url.clone(),
                });
            }
            let mut current_y = line_y;
            for (idx, line) in wrapped.iter().enumerate() {
                let y = current_y + font_size as f32 + line_height * idx as f32;
                if let Some((start, end, jira_key)) = find_jira_in_line(line) {
                    let before = &line[..start];
                    let key_text = &line[start..end];
                    let after = &line[end..];

                    let before_width = measure_text(before, None, font_size, 1.0).width;
                    let key_width = measure_text(key_text, None, font_size, 1.0).width;
                    let key_rect = Rect::new(
                        text_x + before_width,
                        y - font_size as f32,
                        key_width,
                        line_height,
                    );

                    let (mx, my) = mouse_position();
                    let hover = point_in_rect(vec2(mx, my), key_rect);
                    let key_color = if hover {
                        Color::new(1.0, 1.0, 1.0, 1.0)
                    } else {
                        ctx.theme.active_color
                    };

                    draw_text(
                        before,
                        text_x,
                        y,
                        font_size as f32,
                        Color::new(1.0, 1.0, 1.0, 1.0),
                    );
                    draw_text(
                        key_text,
                        text_x + before_width,
                        y,
                        font_size as f32,
                        key_color,
                    );
                    draw_text(
                        after,
                        text_x + before_width + key_width,
                        y,
                        font_size as f32,
                        Color::new(1.0, 1.0, 1.0, 1.0),
                    );

                    hits.push(PrHit {
                        rect: key_rect,
                        url: format!("https://gspcloud.atlassian.net/browse/{}", jira_key),
                    });
                } else {
                    draw_text(
                        line,
                        text_x,
                        y,
                        font_size as f32,
                        Color::new(1.0, 1.0, 1.0, 1.0),
                    );
                }
            }
            current_y += line_height * wrapped.len() as f32;

            line_y = current_y + item_gap;
        }
        hits
    })
}

fn draw_loader_indicator(layout: ClockLayout) {
    FRAME_CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        let size = layout.board_grid.step() * 3.0;
        let padding = layout.left_x - ctx.container.x;
        let mut x = ctx.container.x + ctx.container.w - size - padding;
        let mut y = ctx.container.y + ctx.container.h - size - padding;
        x = snap_to_grid(ctx.container.x, x, layout.board_grid.step());
        y = snap_to_grid(ctx.container.y, y, layout.board_grid.step());

        // Soft bubble pulses around the loader.
        let t = get_time() as f32;
        let offsets = [vec2(-0.6, 0.08), vec2(0.45, -0.45), vec2(0.6, 0.5)];
        let angle = t * 1.4;
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        for (idx, offset) in offsets.iter().enumerate() {
            let phase = t * 0.9 + idx as f32 * 1.3;
            let pulse = (phase.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
            let bubble_size = (layout.board_grid.cell * (0.5 + pulse * 0.5)).max(2.0);
            let alpha = (0.15 + pulse * 0.35).min(0.5);
            let rot_x = offset.x * cos_a - offset.y * sin_a;
            let rot_y = offset.x * sin_a + offset.y * cos_a;
            let bx = x + size * 0.5 + rot_x * size * 0.45 - bubble_size * 0.5;
            let by = y + size * 0.5 + rot_y * size * 0.45 - bubble_size * 0.5;
            draw_rectangle(
                bx,
                by,
                bubble_size,
                bubble_size,
                Color::new(1.0, 1.0, 1.0, alpha),
            );
        }
    });
}

fn glyph_pattern(ch: char) -> [&'static str; 7] {
    match ch {
        // Pixel Operator-inspired 5x7 glyphs.
        '0' => [
            ".###.", "#...#", "#..##", "#.#.#", "##..#", "#...#", ".###.",
        ],
        '1' => [
            "..#..", ".##..", "..#..", "..#..", "..#..", "..#..", ".###.",
        ],
        '2' => [
            ".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####",
        ],
        '3' => [
            ".###.", "#...#", "....#", "..##.", "....#", "#...#", ".###.",
        ],
        '4' => [
            "...#.", "..##.", ".#.#.", "#..#.", "#####", "...#.", "...#.",
        ],
        '5' => [
            "#####", "#....", "####.", "....#", "....#", "#...#", ".###.",
        ],
        '6' => [
            ".###.", "#...#", "#....", "####.", "#...#", "#...#", ".###.",
        ],
        '7' => [
            "#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#...",
        ],
        '8' => [
            ".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###.",
        ],
        '9' => [
            ".###.", "#...#", "#...#", ".####", "....#", "#...#", ".###.",
        ],
        ':' => ["...", ".#.", ".#.", "...", ".#.", ".#.", "..."],
        'A' => [
            ".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
        ],
        'B' => [
            "####.", "#...#", "#...#", "####.", "#...#", "#...#", "####.",
        ],
        'C' => [
            ".###.", "#...#", "#....", "#....", "#....", "#...#", ".###.",
        ],
        'D' => [
            "####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####.",
        ],
        'E' => [
            "#####", "#....", "#....", "####.", "#....", "#....", "#####",
        ],
        'F' => [
            "#####", "#....", "#....", "####.", "#....", "#....", "#....",
        ],
        'G' => [
            ".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".###.",
        ],
        'H' => [
            "#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
        ],
        'I' => [
            "#####", "..#..", "..#..", "..#..", "..#..", "..#..", "#####",
        ],
        'J' => [
            "..###", "...#.", "...#.", "...#.", "...#.", "#..#.", ".##..",
        ],
        'K' => [
            "#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#",
        ],
        'L' => [
            "#....", "#....", "#....", "#....", "#....", "#....", "#####",
        ],
        'M' => [
            "#...#", "##.##", "#.#.#", "#...#", "#...#", "#...#", "#...#",
        ],
        'N' => [
            "#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#", "#...#",
        ],
        'O' => [
            ".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
        ],
        'P' => [
            "####.", "#...#", "#...#", "####.", "#....", "#....", "#....",
        ],
        'Q' => [
            ".###.", "#...#", "#...#", "#...#", "#.#.#", "#..#.", ".##.#",
        ],
        'R' => [
            "####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#",
        ],
        'S' => [
            ".###.", "#....", "#....", ".###.", "....#", "....#", "###..",
        ],
        'T' => [
            "#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#..",
        ],
        'U' => [
            "#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
        ],
        'V' => [
            "#...#", "#...#", "#...#", "#...#", "#...#", ".#.#.", "..#..",
        ],
        'W' => [
            "#...#", "#...#", "#...#", "#.#.#", "#.#.#", "##.##", "#...#",
        ],
        'X' => [
            "#...#", ".#.#.", "..#..", "..#..", "..#..", ".#.#.", "#...#",
        ],
        'Y' => [
            "#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#..",
        ],
        'Z' => [
            "#####", "....#", "...#.", "..#..", ".#...", "#....", "#####",
        ],
        ' ' => [
            ".....", ".....", ".....", ".....", ".....", ".....", ".....",
        ],
        _ => [
            ".....", ".....", ".....", ".....", ".....", ".....", ".....",
        ],
    }
}

#[macroquad::main(conf)]
async fn main() {
    let _ = dotenvy::dotenv();
    let accent_palette = [
        Color::new(0.09, 0.42, 0.2, 1.0),
        Color::new(0.19, 0.63, 0.31, 1.0),
        Color::new(0.25, 0.77, 0.39, 1.0),
        Color::new(0.61, 0.91, 0.66, 1.0),
        Color::new(0.18, 0.53, 0.88, 1.0),
        Color::new(0.44, 0.67, 0.96, 1.0),
        Color::new(0.96, 0.68, 0.24, 1.0),
        Color::new(0.95, 0.55, 0.4, 1.0),
        Color::new(0.78, 0.56, 0.95, 1.0),
        Color::new(0.88, 0.45, 0.74, 1.0),
    ];

    let mut accent_index = 0usize;
    let mut hour_format = HourFormat::H24;
    let mut time_format = TimeFormat::HhMmSs;
    let mut github_status = ConnectionStatus::Unknown;
    let mut github_rx: Option<mpsc::Receiver<GithubFetchResult>> = None;
    let mut github_last_fetch = Local::now().timestamp() - 360;
    let mut github_token = load_github_token();
    let config_opened = github_token.is_none();
    if github_token.is_none() {
        ensure_config_file();
    }
    let mut github_prs: Vec<GithubPr> = Vec::new();
    let github_icon = load_github_icon_texture(96);
    let pr_icon = load_pr_icon_texture(96);
    let mut last_token_check = 0i64;

    loop {
        let accent = accent_palette[accent_index];
        let theme = Theme {
            background_color: Color::new(0.06, 0.07, 0.08, 1.0),
            inactive_color: Color::new(0.12, 0.13, 0.15, 1.0),
            active_color: accent,
            noise_color: accent,
            active_alpha: 0.82,
            active_alpha_jitter: 0.4,
        };

        let container = Rect::new(0.0, 0.0, screen_width(), screen_height());
        update_context(theme, container);

        let now = Local::now();
        let time_string = format_time(hour_format, time_format);
        let am_pm = am_pm_suffix(hour_format);
        let date_string = format_day_month();
        let year_string = format_year();

        if config_opened && github_token.is_none() && now.timestamp() - last_token_check >= 2 {
            last_token_check = now.timestamp();
            github_token = load_github_token();
        }

        if now.timestamp() - github_last_fetch >= 300 && github_rx.is_none() {
            github_last_fetch = now.timestamp();
            if let Some(token) = github_token.clone() {
                github_status = ConnectionStatus::Unknown;
                github_rx = Some(spawn_github_fetch(token));
            } else {
                github_status = ConnectionStatus::Disconnected;
                github_prs.clear();
            }
        }

        if let Some(rx) = &github_rx {
            if let Ok(result) = rx.try_recv() {
                github_status = if result.connected {
                    ConnectionStatus::Connected
                } else {
                    ConnectionStatus::Disconnected
                };
                github_prs = result.prs;
                github_rx = None;
            }
        }

        let layout = draw_clock(
            &year_string,
            &date_string,
            &time_string,
            am_pm.as_deref(),
            now.minute() as i32,
        );

        let button_grid = grid_from_height(42.0, 0.25);
        let button_rect = github_button_rect(container, button_grid);
        draw_github_button(github_status, github_icon.as_ref(), button_rect);

        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            if point_in_rect(vec2(mx, my), button_rect) {
                github_token = load_github_token();
                if let Some(token) = github_token.clone() {
                    github_status = ConnectionStatus::Unknown;
                    github_rx = Some(spawn_github_fetch(token));
                } else {
                    github_status = ConnectionStatus::Disconnected;
                    github_prs.clear();
                }
            }
        }

        let pr_hits = if github_prs.is_empty() {
            Vec::new()
        } else {
            draw_pr_list(&github_prs, pr_icon.as_ref(), layout)
        };

        if github_rx.is_some() {
            draw_loader_indicator(layout);
        }

        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            let point = vec2(mx, my);
            for hit in pr_hits.iter() {
                if point_in_rect(point, hit.rect) {
                    open_url(&hit.url);
                    break;
                }
            }
        }

        if is_key_pressed(KeyCode::F) {
            time_format = match time_format {
                TimeFormat::HhMmSs => TimeFormat::HhMm,
                TimeFormat::HhMm => TimeFormat::MmSs,
                TimeFormat::MmSs => TimeFormat::IsoTime,
                TimeFormat::IsoTime => TimeFormat::HhMmSs,
            };
        }
        if is_key_pressed(KeyCode::H) {
            hour_format = if hour_format == HourFormat::H24 {
                HourFormat::H12
            } else {
                HourFormat::H24
            };
        }
        if is_key_pressed(KeyCode::C) {
            accent_index = (accent_index + 1) % accent_palette.len();
        }

        next_frame().await;
    }
}
