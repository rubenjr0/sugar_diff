use std::fmt::Display;

use color_eyre::Result;

#[derive(Debug)]
struct Time {
    h: u8,
    m: u8,
}

impl From<String> for Time {
    fn from(s: String) -> Self {
        let mut fields = s.split(':').map(|t| t.parse());
        let h = fields.next().unwrap().unwrap();
        let m = fields.next().unwrap().unwrap();
        Self { h, m }
    }
}

impl Time {
    fn timestamp(&self) -> i16 {
        self.h as i16 * 60 + self.m as i16
    }
}

impl Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}:{:02}", self.h, self.m)
    }
}

#[derive(Debug)]
pub struct Meassurement {
    y: i16,
    t: Time,
}

impl Meassurement {
    pub fn new(ys: String, t: String) -> Result<Meassurement> {
        let y = ys.trim().parse()?;
        let t = t.trim().to_string().into();
        Ok(Meassurement { y, t })
    }

    pub fn y(&self) -> i16 {
        self.y
    }

    pub fn timestamp(&self) -> i16 {
        self.t.timestamp()
    }

    pub fn diff(&self, m_2: &Meassurement) -> f32 {
        let dy = (self.y - m_2.y) as f32;
        let dt = (self.timestamp() - m_2.timestamp()) as f32;
        dy / dt
    }
}

impl ToString for Meassurement {
    fn to_string(&self) -> String {
        format!("[{}] {}", self.t, self.y)
    }
}
