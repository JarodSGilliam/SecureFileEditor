pub struct InsertionPoint {
    pub x: usize,
    pub y: usize,
}

impl InsertionPoint {
    pub fn new() -> InsertionPoint {
        InsertionPoint { x: 0, y: 0 }
    }

    pub fn clone(&self) -> InsertionPoint {
        InsertionPoint {
            x: self.x,
            y: self.y,
        }
    }
}
