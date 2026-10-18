//! open-scrantic: a modern reimplementation of Screen Antics: Johnny Castaway (Sierra/Dynamix, 1992).
//! Runs on the original, installed screensaver files (SCRANTIC.SCR, RESOURCE.MAP, RESOURCE.001).

use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Fullscreen, Window, WindowId};

use open_scrantic::engine::config::Config;
use open_scrantic::engine::gfx::{HEIGHT, WIDTH};
use open_scrantic::engine::{Clock, Engine, STEP_MS, TICK_MS};
use open_scrantic::present;
use open_scrantic::{assets, audio};

/// The title screen stays up while the 1992 machine loaded the first session.
const INTRO_HOLD: Duration = Duration::from_secs(2);

const USAGE: &str = "\
Usage: open-scrantic [--fullscreen] [--dedither] [--soft-audio]

Options:
  --fullscreen    start fullscreen instead of in a window
  --dedither      blend the 1992 checkerboard dithering into solid colors
  --soft-audio    improve the sounds: resample to the device rate, hide the 8-bit hiss, fade cuts
  -V, --version   print the version and exit
  -h, --help      print this help and exit

The original screensaver files (SCRANTIC.SCR, RESOURCE.MAP, RESOURCE.001) and SCRANTIC.INI are read
from the directory of this executable. Esc exits.";

struct Options {
    fullscreen: bool,
    dedither: bool,
    soft_audio: bool,
}

/// Parses the command line; `Ok(None)` means a message was printed and the program should exit.
fn parse_args() -> Result<Option<Options>, String> {
    let mut o = Options {
        fullscreen: false,
        dedither: false,
        soft_audio: false,
    };
    for a in std::env::args().skip(1) {
        match a.as_str() {
            "--fullscreen" => o.fullscreen = true,
            "--dedither" => o.dedither = true,
            "--soft-audio" => o.soft_audio = true,
            "-V" | "--version" => {
                println!("open-scrantic {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(None);
            }
            _ => return Err(format!("unknown argument {a}\n\n{USAGE}")),
        }
    }
    Ok(Some(o))
}

struct App {
    engine: Engine,
    audio: Option<audio::Audio>,
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    start: Instant,
    steps: u64,
    ticks: u64,
    fullscreen: bool,
    dedither: bool,
    /// The title screen has been requested once; until the first step it needs no further redraws.
    intro_drawn: bool,
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
        let px = &self.engine.frame().px;
        let rgb = if self.dedither {
            present::dedither(px, &self.engine.palette)
        } else {
            present::plain(px, &self.engine.palette)
        };
        present::scale(&rgb, &mut buf, w, h);
        let _ = buf.present();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let mut attrs = Window::default_attributes()
            .with_title("Johnny Castaway")
            .with_min_inner_size(PhysicalSize::new(WIDTH as u32, HEIGHT as u32));
        if self.fullscreen {
            attrs = attrs.with_fullscreen(Some(Fullscreen::Borderless(None)));
        } else {
            // Largest whole multiple of the original 640x480 that fills at most 80% of the screen.
            let k = event_loop
                .primary_monitor()
                .or_else(|| event_loop.available_monitors().next())
                .map_or(1, |m| {
                    let s = m.size();
                    (s.width * 4 / 5 / WIDTH as u32)
                        .min(s.height * 4 / 5 / HEIGHT as u32)
                        .max(1)
                });
            attrs = attrs.with_inner_size(PhysicalSize::new(WIDTH as u32 * k, HEIGHT as u32 * k));
        }
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Rc::new(w),
            Err(e) => {
                eprintln!("error: cannot create window: {e}");
                event_loop.exit();
                return;
            }
        };
        window.set_cursor_visible(!self.fullscreen);
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
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && event.logical_key == Key::Named(NamedKey::Escape) =>
            {
                event_loop.exit();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            return;
        }
        let stepped = self.advance();
        if (stepped || !self.intro_drawn)
            && let Some(w) = &self.window
        {
            self.intro_drawn = true;
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
    let opts = match parse_args() {
        Ok(Some(o)) => o,
        Ok(None) => return,
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(2);
        }
    };
    let data = match assets::ScreensaverData::load(&open_scrantic::app_dir()) {
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
    let audio = audio::Audio::new(opts.soft_audio);
    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    let mut app = App {
        engine,
        audio,
        window: None,
        surface: None,
        start: Instant::now(),
        steps: 0,
        ticks: 0,
        fullscreen: opts.fullscreen,
        dedither: opts.dedither,
        intro_drawn: false,
    };
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("error: {e}");
    }
}
