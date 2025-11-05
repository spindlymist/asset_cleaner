use imgui::Ui;

#[allow(dead_code)]
pub trait ImguiExt {
    fn text_centered<T: AsRef<str>>(&self, text: T);
    fn move_cursor(&self, delta: [f32; 2]);
    fn move_cursor_up(&self, delta: f32);
    fn move_cursor_down(&self, delta: f32);
    fn move_cursor_left(&self, delta: f32);
    fn move_cursor_right(&self, delta: f32);
    fn set_cursor_x(&self, x: f32);
    fn set_cursor_y(&self, y: f32);
    fn cursor_x(&self) -> f32;
    fn cursor_y(&self) -> f32;
}

impl ImguiExt for Ui {
    fn text_centered<T: AsRef<str>>(&self, text: T) {
        let [width_avail, _] = self.content_region_avail();
        let [width_text, _] = self.calc_text_size(&text);
        let x = (width_avail - width_text) / 2.0;
        self.set_cursor_x(x);
        self.text(text);
    }
    
    fn move_cursor(&self, delta: [f32; 2]) {
        let mut pos = self.cursor_pos();
        pos[0] += delta[0];
        pos[1] += delta[1];
        self.set_cursor_pos(pos);
    }
    
    fn move_cursor_up(&self, delta: f32) {
        let mut pos = self.cursor_pos();
        pos[1] -= delta;
        self.set_cursor_pos(pos);
    }

    fn move_cursor_down(&self, delta: f32) {
        let mut pos = self.cursor_pos();
        pos[1] += delta;
        self.set_cursor_pos(pos);
    }
    
    fn move_cursor_left(&self, delta: f32) {
        let mut pos = self.cursor_pos();
        pos[0] -= delta;
        self.set_cursor_pos(pos);
    }

    fn move_cursor_right(&self, delta: f32) {
        let mut pos = self.cursor_pos();
        pos[0] += delta;
        self.set_cursor_pos(pos);
    }
    
    fn set_cursor_x(&self, x: f32) {
        let mut pos = self.cursor_pos();
        pos[0] = x;
        self.set_cursor_pos(pos);
    }
    
    fn set_cursor_y(&self, y: f32) {
        let mut pos = self.cursor_pos();
        pos[1] = y;
        self.set_cursor_pos(pos);
    }
    
    fn cursor_x(&self) -> f32 {
        self.cursor_pos()[0]
    }
    
    fn cursor_y(&self) -> f32 {
        self.cursor_pos()[1]
    }
}
