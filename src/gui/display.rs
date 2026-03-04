#[derive(Debug, Clone)]
pub enum Message {}

pub struct Display {
    buffer: [[bool; 64]; 32], // CHIP-8 display is 64 x 32
    cache: iced::widget::canvas::Cache,
}

impl Display {
    pub fn new() -> Self {
        Self {
            buffer: [[false; 64]; 32],
            cache: iced::widget::canvas::Cache::default(),
        }
    }

    pub fn view(&self) -> iced::Element<'_, Message> {
        iced::widget::Canvas::new(self)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .into()
    }

    pub fn update(&mut self, new_disp: [[bool; 64]; 32]) {
        let mut changed = false;
        for y in 0..32 {
            for x in 0..64 {
                if new_disp[y][x] != self.buffer[y][x] {
                    self.buffer[y][x] = new_disp[y][x];
                    changed = true;
                }
            }
        }
        if changed {
            self.cache.clear();
        }
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
            let w: f32 = frame.width() / 64.0;
            let h: f32 = frame.height() / 32.0;

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
