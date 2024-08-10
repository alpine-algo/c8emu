#[derive(Debug, Clone)]
pub enum Message {}

pub struct Display {
    buffer: [[bool; 64]; 32], // CHIP-8 display is 64 x 32
    cache: iced::widget::canvas::Cache,
}

impl Display {
    pub fn new() -> Self {
        let mut display = Self {
            buffer: [[false; 64]; 32],
            cache: iced::widget::canvas::Cache::default(),
        };
        display.draw_test_pattern();
        display
    }

    pub fn view(&self) -> iced::Element<Message> {
        iced::widget::Canvas::new(self)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .into()
    }

    pub fn draw_test_pattern(&mut self) {
        // [y][x] --> max: [31, 63]]
        self.buffer[5][5] = true;
        self.buffer[9][13] = true;
        self.buffer[10][10] = true;
        self.buffer[15][15] = true;
        self.buffer[27][60] = true;
        self.buffer[19][38] = true;
        self.buffer[30][45] = true;
    }
}

impl<Message> iced::widget::canvas::Program<Message> for Display {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<iced::widget::canvas::Geometry> {
        let screen = self.cache.draw(renderer, bounds.size(), |frame| {
            let w: f32 = 7.0;
            let h: f32 = 7.0;

            for (y, row) in self.buffer.iter().enumerate() {
                for (x, &cell) in row.iter().enumerate() {
                    if cell {
                        let path = iced::widget::canvas::Path::rectangle(
                            iced::Point::new(x as f32 * w, y as f32 * h),
                            iced::Size::new(w, h),
                        );
                        frame.fill(&path, iced::Color::BLACK);
                    } else {
                        // Fill empty cells light gray
                        let path = iced::widget::canvas::Path::rectangle(
                            iced::Point::new(x as f32 * w, y as f32 * h),
                            iced::Size::new(w, h),
                        );
                        frame.fill(&path, iced::Color::from_rgb(0.95, 0.95, 0.95));
                    }
                }
            }
        });

        vec![screen]
    }
}
