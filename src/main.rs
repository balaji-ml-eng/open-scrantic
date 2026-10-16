//! open-scrantic: a modern reimplementation of Screen Antics: Johnny Castaway (Sierra/Dynamix, 1993).
//! Runs on the original, installed game files (SCRANTIC.SCR, RESOURCE.MAP, RESOURCE.001).

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Fullscreen, Window, WindowId};

use open_scrantic::engine::config::Config;
use open_scrantic::engine::gfx::{HEIGHT, WIDTH};
use open_scrantic::engine::{Clock, Engine, STEP_MS, TICK_MS};
use open_scrantic::{assets, audio};

/// The title screen stays up while the 1993 machine loaded the first session.
const INTRO_HOLD: Duration = Duration::from_secs(2);

const USAGE: &str = "\
Usage: open-scrantic [OPTIONS] [GAME_DIR]

GAME_DIR is the installed game directory containing SCRANTIC.SCR, RESOURCE.MAP and
RESOURCE.001 (default: current directory).

Options:
  -f, --fullscreen   start in fullscreen (F11 toggles)
  -s, --saver        screensaver mode: fullscreen, exit on any input
      --scale N      initial window scale (default 1)
      --no-sound     disable sound effects
  -h, --help         show this help";

struct Args {
    dir: PathBuf,
    fullscreen: bool,
    saver: bool,
    scale: u32,
    sound: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        dir: PathBuf::from("."),
        fullscreen: false,
        saver: false,
        scale: 1,
        sound: true,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "-f" | "--fullscreen" => args.fullscreen = true,
            "-s" | "--saver" => {
                args.saver = true;
                args.fullscreen = true;
            }
            "--no-sound" => args.sound = false,
            "--scale" => {
                args.scale = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .filter(|&s| (1..=8).contains(&s))
                    .ok_or("--scale expects 1..8")?
            }
            "-h" | "--help" => return Err(USAGE.to_string()),
            _ if a.starts_with('-') => return Err(format!("unknown option {a}\n\n{USAGE}")),
            _ => args.dir = PathBuf::from(a),
        }
    }
    Ok(args)
}

struct App {
    args: Args,
    engine: Engine,
    audio: Option<audio::Audio>,
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    start: Instant,
    steps: u64,
    ticks: u64,
    first_cursor: Option<PhysicalPosition<f64>>,
}

impl App {
    /// Advances the virtual Windows 3.1 clocks: a story step every 55 ms and the 16 ms tick.
    fn advance(&mut self) -> bool {
        let elapsed = self.start.elapsed().as_millis() as u64;
        let due = elapsed / STEP_MS;
        let mut stepped = false;
        // After long stalls (sleep, debugger) resume instead of fast-forwarding.
        if due > self.steps + 20 {
            self.steps = due - 1;
            self.ticks = self.steps * STEP_MS / TICK_MS;
        }
        while self.steps < due {
            self.steps += 1;
            let virtual_ms = self.steps * STEP_MS;
            while (self.ticks + 1) * TICK_MS <= virtual_ms {
                self.ticks += 1;
                self.engine.tick();
            }
            self.engine.step(&Clock::now());
            for id in self.engine.take_sounds() {
                if let Some(a) = self.audio.as_mut().filter(|_| self.engine.config.sounds) {
                    a.play(self.engine.sound_bank(), id);
                }
            }
            stepped = true;
        }
        stepped
    }

    fn redraw(&mut self) {
        let (Some(window), Some(surface)) = (&self.window, &mut self.surface) else {
            return;
        };
        let size = window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        if surface.resize(w, h).is_err() {
            return;
        }
        let Ok(mut buf) = surface.buffer_mut() else { return };
        let (w, h) = (size.width as usize, size.height as usize);
        let scale = (w as f64 / WIDTH as f64).min(h as f64 / HEIGHT as f64);
        let (dw, dh) = ((WIDTH as f64 * scale) as usize, (HEIGHT as f64 * scale) as usize);
        let (ox, oy) = ((w - dw) / 2, (h - dh) / 2);
        let pal: Vec<u32> = self
            .engine
            .palette
            .iter()
            .map(|c| (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32)
            .collect();
        let cols: Vec<usize> = (0..dw)
            .map(|x| (x * WIDTH as usize / dw).min(WIDTH as usize - 1))
            .collect();
        let px = &self.engine.frame().px;
        buf.fill(0);
        for y in 0..dh {
            let src = (y * HEIGHT as usize / dh).min(HEIGHT as usize - 1) * WIDTH as usize;
            let dst = (y + oy) * w + ox;
            for (x, &sx) in cols.iter().enumerate() {
                buf[dst + x] = pal[px[src + sx] as usize];
            }
        }
        let _ = buf.present();
    }

    fn toggle_fullscreen(&self) {
        if let Some(win) = &self.window {
            win.set_fullscreen(if win.fullscreen().is_some() {
                None
            } else {
                Some(Fullscreen::Borderless(None))
            });
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Johnny Castaway")
            .with_inner_size(LogicalSize::new(
                WIDTH as u32 * self.args.scale,
                HEIGHT as u32 * self.args.scale,
            ))
            .with_fullscreen(self.args.fullscreen.then_some(Fullscreen::Borderless(None)));
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Rc::new(w),
            Err(e) => {
                eprintln!("error: cannot create window: {e}");
                event_loop.exit();
                return;
            }
        };
        if self.args.saver {
            window.set_cursor_visible(false);
        }
        let surface = softbuffer::Context::new(Rc::clone(&window))
            .and_then(|ctx| softbuffer::Surface::new(&ctx, Rc::clone(&window)));
        match surface {
            Ok(s) => self.surface = Some(s),
            Err(e) => {
                eprintln!("error: cannot create drawing surface: {e}");
                event_loop.exit();
                return;
            }
        }
        self.window = Some(window);
        self.start = Instant::now() + INTRO_HOLD;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if self.args.saver {
                    event_loop.exit();
                    return;
                }
                match event.logical_key {
                    Key::Named(NamedKey::Escape) => event_loop.exit(),
                    Key::Named(NamedKey::F11) => self.toggle_fullscreen(),
                    Key::Character(c) if c.eq_ignore_ascii_case("q") => event_loop.exit(),
                    Key::Character(c) if c.eq_ignore_ascii_case("f") => self.toggle_fullscreen(),
                    _ => {}
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                ..
            } if self.args.saver => event_loop.exit(),
            WindowEvent::CursorMoved { position, .. } if self.args.saver => {
                let first = *self.first_cursor.get_or_insert(position);
                if (position.x - first.x).abs() + (position.y - first.y).abs() > 8.0 {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            return;
        }
        if (self.advance() || self.steps == 0)
            && let Some(w) = &self.window
        {
            w.request_redraw();
        }
        let next = self.start + Duration::from_millis((self.steps + 1) * STEP_MS);
        event_loop.set_control_flow(ControlFlow::WaitUntil(next));
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        if let Err(e) = self.engine.config.save() {
            eprintln!("warning: cannot save settings: {e}");
        }
    }
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(msg) => {
            println!("{msg}");
            std::process::exit(if msg == USAGE { 0 } else { 2 });
        }
    };
    let data = match assets::GameData::load(&args.dir) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let engine = match Engine::new(data, Config::load(), seed) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    let audio = if args.sound { audio::Audio::new() } else { None };
    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    let mut app = App {
        args,
        engine,
        audio,
        window: None,
        surface: None,
        start: Instant::now(),
        steps: 0,
        ticks: 0,
        first_cursor: None,
    };
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("error: {e}");
    }
}
