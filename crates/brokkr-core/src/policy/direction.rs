//! Table order, shared by admission, seat returns and the rail (decision 0094).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Return,
    Loop,
}

impl Direction {
    pub fn of(order: &[String], from: &str, next: &str) -> Option<Self> {
        let from = order.iter().position(|phase| phase == from)?;
        let next = order.iter().position(|phase| phase == next)?;
        Some(if next > from {
            Self::Forward
        } else if next < from {
            Self::Return
        } else {
            Self::Loop
        })
    }
}
