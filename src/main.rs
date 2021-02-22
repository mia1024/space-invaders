#[macro_use]
extern crate lazy_static;

use std::io::{stdout, Write, Stdout};
use std::ops::Index;

use crossterm;
use crossterm::{cursor, execute, queue, QueueableCommand, terminal::{
    Clear, ClearType, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, SetTitle,
}, event::{KeyCode, KeyEvent, poll, read, Event, KeyModifiers,KeyCode::Char}, style::{Attributes,
  Color,SetBackgroundColor, SetForegroundColor, SetAttributes, SetAttribute, Print, ResetColor, Attribute},
                cursor::{DisableBlinking, EnableBlinking, Show, Hide, MoveTo}
};
use std::time::{Duration};
use std::thread::sleep;
use std::process::exit;


static SPRITE: &str = "
         ███████╗ ██████╗   █████╗   ██████╗ ███████╗
         ██╔════╝ ██╔══██╗ ██╔══██╗ ██╔════╝ ██╔════╝
         ███████╗ ██████╔╝ ███████║ ██║      █████╗
         ╚════██║ ██╔═══╝  ██╔══██║ ██║      ██╔══╝
         ███████║ ██║      ██║  ██║ ╚██████╗ ███████╗
         ╚══════╝ ╚═╝      ╚═╝  ╚═╝  ╚═════╝ ╚══════╝

██╗ ███╗   ██╗ ██╗   ██╗  █████╗  ██████╗  ███████╗ ██████╗  ███████╗
██║ ████╗  ██║ ██║   ██║ ██╔══██╗ ██╔══██╗ ██╔════╝ ██╔══██╗ ██╔════╝
██║ ██╔██╗ ██║ ██║   ██║ ███████║ ██║  ██║ █████╗   ██████╔╝ ███████╗
██║ ██║╚██╗██║ ╚██╗ ██╔╝ ██╔══██║ ██║  ██║ ██╔══╝   ██╔══██╗ ╚════██║
██║ ██║ ╚████║  ╚████╔╝  ██║  ██║ ██████╔╝ ███████╗ ██║  ██║ ███████║
╚═╝ ╚═╝  ╚═══╝   ╚═══╝   ╚═╝  ╚═╝ ╚═════╝  ╚══════╝ ╚═╝  ╚═╝ ╚══════╝

                          V 0.0.1, Mia Celeste



                         PRESS ENTER TO START
                         PRESS CTRL-C TO QUIT
"; // texts generated with https://patorjk.com/software/taag/

type px = usize;

#[derive(Debug)]
pub struct Config {
    pub screen_width: px,
    pub screen_height: px,
    pub framerate: px,
    pub missile_speed: px,
    pub player_speed: px,
    pub player_lives: px,
    pub alien_speed: px,
    pub alien_rows: usize,
    pub alien_counts: usize,
    pub alien_missile_frequency: f32,
}


fn load_config() -> Config {
    let config = ini::Ini::load_from_file("config.ini").unwrap();
    let mut conf = Config {
        screen_width: 79,
        screen_height: 24,
        framerate: 10,
        missile_speed: 1,
        player_speed: 2,
        player_lives: 3,
        alien_speed: 1,
        alien_rows: 5,
        alien_counts: 11,
        alien_missile_frequency: 0.0001,
    };
    for (sec, prop) in config.iter() {
        match sec.unwrap() {
            "Game" => {
                for (key, val) in prop.iter() {
                    match key {
                        "screen_width" => conf.screen_width = val.parse().unwrap(),
                        "screen_height" => conf.screen_height = val.parse().unwrap(),
                        "framerate" => conf.framerate = val.parse().unwrap(),
                        "missile_speed" => conf.missile_speed = val.parse().unwrap(),
                        _ => { /* extra key is ignored */ }
                    }
                }
            }
            "Player" => {
                for (key, val) in prop.iter() {
                    match key {
                        "speed" => conf.player_speed = val.parse().unwrap(),
                        "lives" => conf.player_lives = val.parse().unwrap(),
                        _ => { /* extra key is ignored */ }
                    }
                }
            }
            "Alien" => {
                for (key, val) in prop.iter() {
                    match key {
                        "speed" => conf.alien_speed = val.parse().unwrap(),
                        "rows" => conf.alien_rows = val.parse().unwrap(),
                        "counts" => conf.alien_counts = val.parse().unwrap(),
                        "missile_frequency" => conf.alien_missile_frequency = val.parse().unwrap(),
                        _ => { /* extra key is ignored */ }
                    }
                }
            }
            _ => { /* extra key is ignored */ }
        }
    }
    conf
}

lazy_static! {
    pub static ref CONFIG:Config = load_config();
}

struct Framebuffer {
    buf: Vec<Pixel>,
}

#[derive(Clone)]
struct Pixel {
    pub char: String,
    pub bg: Color,
    pub fg: Color,
    pub attrs: Attributes,
}

const DEFAULT_BG: Color = Color::AnsiValue(7);
const DEFAULT_FG: Color = Color::Black;


impl Pixel {
    fn default() -> Pixel {
        Pixel {
            char: String::from(" "),
            bg: DEFAULT_BG,
            fg: DEFAULT_FG,
            attrs: Default::default(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Hitbox {
    pub width: px,
    pub height: px,
}

trait Hittable {
    fn hit(&self, x: px, y: px) -> bool;
}

#[derive(Copy, Clone, Debug)]
struct Player {
    pub x: px,
    pub y: px,
    pub hitbox: Hitbox,
}

impl Hittable for Player {
    fn hit(&self, x: px, y: px) -> bool {
        self.x <= x && x <= self.x + self.hitbox.width &&
            self.y <= y && y <= self.y + self.hitbox.height
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum MissileDirection {
    Up,
    Down,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum AlienDirection {
    Left,
    Right,
}

#[derive(Copy, Clone, Debug)]
struct Missile {
    pub x: px,
    pub y: px,
    pub hitbox: Hitbox,
    pub direction: MissileDirection,
}

#[derive(Copy, Clone, Debug)]
struct Alien {
    pub x: px,
    pub y: px,
    pub hitbox: Hitbox,
    pub value: i32,
}

impl Hittable for Alien {
    fn hit(&self, x: px, y: px) -> bool {
        self.x <= x && x <= self.x + self.hitbox.width &&
            self.y <= y && y <= self.y + self.hitbox.height
    }
}

struct Game {
    pub aliens: Vec<Alien>,
    pub alien_direction: AlienDirection,
    pub alien_level: i32,
    pub missiles: Vec<Missile>,
    pub player: Player,
    pub lives_left: i32,
    pub score: i32,
}

impl Game {
    fn new() -> Game {
        let mut aliens = Vec::<Alien>::new();
        let left_beg = (CONFIG.screen_width - CONFIG.alien_counts * 3) / 2;
        for y in 0..CONFIG.alien_rows {
            for x in 0..CONFIG.alien_counts {
                aliens.push(Alien {
                    x: x * 3 + 1 + left_beg,
                    y: y * 2 + 1,
                    hitbox: Hitbox { width: 2, height: 1 },
                    value: ((CONFIG.alien_rows - y) * 100) as i32,
                })
            }
        }
        let player = Player {
            x: (CONFIG.screen_width - 5) / 2,
            y: CONFIG.screen_height - 3,
            hitbox: Hitbox { width: 5, height: 2 },
        };
        Game {
            aliens,
            alien_direction: AlienDirection::Left,
            missiles: Vec::<Missile>::new(),
            score: 0,
            lives_left: CONFIG.player_lives as i32,
            player,
            alien_level: 0,
        }
    }
}

impl Framebuffer {
    fn new() -> Framebuffer {
        let buf = vec![Pixel::default(); CONFIG.screen_width * CONFIG.screen_height];
        Framebuffer { buf }
    }

    fn sprite() -> Framebuffer {
        let mut buf = Vec::<Pixel>::with_capacity(CONFIG.screen_width * CONFIG.screen_height);
        let left_fill = (CONFIG.screen_width - 69) / 2; // 69 is the widest part of the sprite
        let top_fill = (CONFIG.screen_height - 22) / 2;
        let mut fill = vec![Pixel::default(); top_fill * CONFIG.screen_width];
        buf.append(&mut fill);
        for line in SPRITE.lines() {
            for _ in 0..left_fill {
                buf.push(Pixel::default())
            }
            let mut i: px = left_fill;
            for char in line.chars() {
                let mut pix = Pixel::default();
                pix.char = char.to_string();
                buf.push(pix);
                i += 1;
            }
            if i < CONFIG.screen_width - 1 {
                for _ in 0..(CONFIG.screen_width - i) {
                    buf.push(Pixel::default());
                }
            }
        }
        for _ in 0..(CONFIG.screen_width * CONFIG.screen_height - buf.len()) {
            buf.push(Pixel::default());
        }
        Framebuffer { buf }
    }

    fn putpixel(&mut self, x: px, y: px, pix: Pixel) { // name inspired by PIL
        self.buf[y * CONFIG.screen_width + x] = pix;
    }

    fn from_game(game: &Game) -> Framebuffer {
        let mut buf = Framebuffer::new();
        for a in game.aliens.iter() {
            let left_pix = Pixel {
                char: String::from("▛"),
                bg: DEFAULT_BG,
                fg: match a.value {
                    100 => Color::Yellow,
                    200 => Color::Cyan,
                    300 => Color::Blue,
                    400 => Color::Magenta,
                    _ => Color::Red
                },
                attrs: Default::default(),
            };
            let mut right_pix = left_pix.clone();
            right_pix.char = String::from("▜");
            buf.putpixel(a.x, a.y, left_pix);
            buf.putpixel(a.x + 1, a.y, right_pix)
        }

        for m in game.missiles.iter() {
            buf.putpixel(m.x, m.y, Pixel {
                char: String::from("┃"),
                bg: DEFAULT_BG,
                fg: { if m.direction == MissileDirection::Down { Color::AnsiValue(13) } else { Color::AnsiValue(2) } },
                attrs: Default::default(),
            })
        }
        for x in 0..game.player.hitbox.width {
            let p = Pixel {
                char: String::from("▀"),
                bg: DEFAULT_BG,
                fg: Color::DarkGreen,
                attrs: Default::default(),
            };
            buf.putpixel(game.player.x + x, game.player.y + 1, p);
        }
        buf.putpixel(game.player.x + 1, game.player.y, Pixel {
            char: String::from("▟"),
            bg: DEFAULT_BG,
            fg: Color::DarkGreen,
            attrs: Default::default(),
        });
        buf.putpixel(game.player.x + 2, game.player.y, Pixel {
            char: String::from("▄"),
            bg: DEFAULT_BG,
            fg: Color::DarkGreen,
            attrs: Default::default(),
        });
        buf.putpixel(game.player.x + 3, game.player.y, Pixel {
            char: String::from("▙"),
            bg: DEFAULT_BG,
            fg: Color::DarkGreen,
            attrs: Default::default(),
        });

        buf
    }

    fn render(&self, stdout: &mut Stdout) {
        stdout.queue(Clear(ClearType::All)).unwrap();
        stdout.queue(cursor::MoveTo(0, 0)).unwrap();
        // too much ANSI escape can cause the terminal to blink,
        // so we try to minimize the amount of them sent out
        for y in 0..(CONFIG.screen_height - 1) {
            let row = &self[y];
            stdout.queue(cursor::MoveTo(0, y as u16)).unwrap();
            let pix = Pixel::default();
            queue!(
                stdout,
                SetForegroundColor(pix.fg),
                SetBackgroundColor(pix.bg),
                SetAttributes(pix.attrs)
            ).unwrap();
            let mut last_bg = pix.bg;
            let mut last_fg = pix.fg;
            let mut last_attrs = pix.attrs;
            for pix in row {
                if last_bg != pix.bg {
                    stdout.queue(SetBackgroundColor(pix.bg)).unwrap();
                    last_bg = pix.bg;
                }
                if last_fg != pix.fg {
                    stdout.queue(SetForegroundColor(pix.fg)).unwrap();
                    last_fg = pix.fg;
                }
                if last_attrs != pix.attrs {
                    stdout.queue(SetAttributes(pix.attrs)).unwrap();
                    last_attrs = pix.attrs;
                }
                stdout.queue(Print(&pix.char)).unwrap();
            }
            queue!(
                stdout,
                ResetColor,
                SetAttribute(Attribute::Reset)
            ).unwrap();
        }
    }
}


impl Index<usize> for Framebuffer {
    type Output = [Pixel];

    fn index(&self, index: usize) -> &Self::Output { // retrieve the slice for a row
        &self.buf[index * CONFIG.screen_width..(index + 1) * CONFIG.screen_width]
    }
}

fn recv_key() -> Option<KeyEvent> {
    let mut e: Option<KeyEvent> = None;
    while poll(Duration::new(0, 0)).unwrap() {
        match read().unwrap() {
            Event::Key(event) => e = Some(event),
            _ => { e = None }
        }
    }
    e
}

fn cleanup(exit_code: i32, message: &str) {
    let mut stdout = stdout();
    disable_raw_mode().unwrap();
    queue!(stdout, LeaveAlternateScreen,EnableBlinking,Show,Print(message)).unwrap();
    stdout.flush().unwrap();
    exit(exit_code);
}

fn main() {
    let mut stdout = stdout();
    execute!(stdout,
        SetTitle("Space Invaders"),
        EnterAlternateScreen,
        DisableBlinking,
        Hide,
        // ANSI resizing doesn't work on my computer and not sure if it works on others
    ).unwrap();
    enable_raw_mode().unwrap();

    let frame = Framebuffer::sprite();
    frame.render(&mut stdout);
    stdout.flush().unwrap();

    loop { // the loading screen
        let key = recv_key();
        match key {
            Some(key) => {
                match key.code {
                    Char(c) => {
                        if c == 'c' && key.modifiers.contains(KeyModifiers::CONTROL) { cleanup(0, ""); };
                    }
                    KeyCode::Enter => {
                        break; // go into mainloop
                    }
                    _ => {}
                }
            }
            _ => { /* Do nothing*/ }
        }
    }

    let mut game = Game::new();
    loop { // the game's mainloop
        let key = recv_key();
        match key {
            Some(key) => {
                match key.code {
                    Char(c) => {
                        if c == 'c' && key.modifiers.contains(KeyModifiers::CONTROL) { cleanup(0, ""); };
                        if c == ' ' {
                            game.missiles.push(Missile {
                                x: game.player.x + 2,
                                y: game.player.y - 1,
                                hitbox: Hitbox { width: 1, height: 1 },
                                direction: MissileDirection::Up,
                            })
                        }
                    }
                    KeyCode::Left => {
                        game.player.x -= CONFIG.player_speed.min(game.player.x);
                    }
                    KeyCode::Right => {
                        let edge = (CONFIG.screen_width) as i32 - (game.player.x + game.player.hitbox.width) as i32;
                        if edge > 0 {
                            game.player.x += CONFIG.player_speed.min(edge as px);
                        }
                    }
                    _ => {}
                }
            }
            _ => { /* Do nothing*/ }
        }
        let mut i = 0;
        while i < game.missiles.len() {
            for j in i..game.missiles.len() {
                let m = game.missiles[i];
                let n = game.missiles[j];
                if (m.y == n.y - 1 || m.y == n.y || m.y == n.y + 1) && m.x == n.x && n.direction != m.direction {
                    game.missiles.remove(j);
                    game.missiles.remove(i);
                    break;
                }
            }
            if i>=game.missiles.len(){
                break;
            }
            let m = &mut game.missiles[i];
            match m.direction {
                MissileDirection::Up => {
                    if m.y <= CONFIG.missile_speed {
                        game.missiles.remove(i);
                    } else {
                        m.y -= CONFIG.missile_speed;
                        let mut j = 0;
                        while j < game.aliens.len() {
                            let a = game.aliens[j];
                            if a.hit(m.x, m.y) {
                                game.aliens.remove(j);
                                game.missiles.remove(i);
                                game.score += a.value * game.alien_level;
                                break;
                            }
                            j += 1;
                        }
                        i += 1;
                    }
                }
                MissileDirection::Down => {
                    if m.y + CONFIG.missile_speed >= CONFIG.screen_height {
                        game.missiles.remove(i);
                    } else {
                        m.y += CONFIG.missile_speed;
                        if game.player.hit(m.x, m.y) {
                            game.lives_left -= 1;
                            game.missiles.clear();
                            break;
                        }
                        i += 1;
                    }
                }
            }
        }

        for a in game.aliens.iter_mut() {
            let distance = CONFIG.alien_speed;
            match game.alien_direction {
                AlienDirection::Left => {
                    a.x -= distance.min(a.x);
                }
                AlienDirection::Right => {
                    a.x += distance.min((CONFIG.screen_width as i32 - distance as i32 - 1).max(0) as px);
                }
            }
        }
        for a in game.aliens.iter() {
            if a.x == 0 || a.x == CONFIG.screen_width-1 {
                game.alien_level += 1;
                for a in game.aliens.iter_mut() {
                    a.y += 1;
                }
                match game.alien_direction {
                    AlienDirection::Left => {
                        game.alien_direction = AlienDirection::Right
                    }
                    AlienDirection::Right => {
                        game.alien_direction = AlienDirection::Left
                    }
                }
                break;
            }
        }

        for a in game.aliens.iter() {
            if rand::random::<f32>() < CONFIG.alien_missile_frequency * game.alien_level as f32 {
                game.missiles.push(Missile {
                    direction: MissileDirection::Down,
                    x: a.x,
                    y: a.y + 1,
                    hitbox: Hitbox { width: 1, height: 1 },
                });
            }
        }

        // render things
        let frame = Framebuffer::from_game(&game);
        frame.render(&mut stdout);
        queue!(
            stdout,
            MoveTo(0,0),
            SetForegroundColor(DEFAULT_FG),
            SetBackgroundColor(DEFAULT_BG),
            SetAttributes(Default::default()),
            Print(format!("Score: {}",game.score)),
            MoveTo((CONFIG.screen_width-10) as u16,0),
            Print(format!("Lives: {}",game.lives_left)),
            ResetColor
        ).unwrap();


        stdout.flush().unwrap();
        if game.lives_left < 0 || game.alien_level > (CONFIG.screen_height - CONFIG.alien_rows * 2) as i32 {
            cleanup(0, format!("You died. Score: {}\n", game.score).as_str());
        }
        if game.aliens.len() == 0 {
            cleanup(0, format!("You won! Score: {}\n", game.score).as_str());
        }
        sleep(Duration::new(0, (1_000_000_000 / CONFIG.framerate) as u32))
        // rust has sub-nanosecond looping time according to my benchmark so the render time
        // can be ignored.
    }
}

