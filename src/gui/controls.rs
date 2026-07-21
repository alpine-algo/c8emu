use iced::keyboard::Key;
use iced::widget::{Column, Container, MouseArea, Row, Scrollable, Text};
use iced::{Alignment, Element, Length};

const SIDEBAR_WIDTH: f32 = 260.0;
const KEY_HEIGHT: f32 = 52.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Binding {
    host_key: char,
    host_label: &'static str,
    chip8_index: usize,
    chip8_label: &'static str,
}

static BINDINGS: [Binding; 16] = [
    Binding {
        host_key: '1',
        host_label: "1",
        chip8_index: 0x1,
        chip8_label: "1",
    },
    Binding {
        host_key: '2',
        host_label: "2",
        chip8_index: 0x2,
        chip8_label: "2",
    },
    Binding {
        host_key: '3',
        host_label: "3",
        chip8_index: 0x3,
        chip8_label: "3",
    },
    Binding {
        host_key: '4',
        host_label: "4",
        chip8_index: 0xC,
        chip8_label: "C",
    },
    Binding {
        host_key: 'Q',
        host_label: "Q",
        chip8_index: 0x4,
        chip8_label: "4",
    },
    Binding {
        host_key: 'W',
        host_label: "W",
        chip8_index: 0x5,
        chip8_label: "5",
    },
    Binding {
        host_key: 'E',
        host_label: "E",
        chip8_index: 0x6,
        chip8_label: "6",
    },
    Binding {
        host_key: 'R',
        host_label: "R",
        chip8_index: 0xD,
        chip8_label: "D",
    },
    Binding {
        host_key: 'A',
        host_label: "A",
        chip8_index: 0x7,
        chip8_label: "7",
    },
    Binding {
        host_key: 'S',
        host_label: "S",
        chip8_index: 0x8,
        chip8_label: "8",
    },
    Binding {
        host_key: 'D',
        host_label: "D",
        chip8_index: 0x9,
        chip8_label: "9",
    },
    Binding {
        host_key: 'F',
        host_label: "F",
        chip8_index: 0xE,
        chip8_label: "E",
    },
    Binding {
        host_key: 'Z',
        host_label: "Z",
        chip8_index: 0xA,
        chip8_label: "A",
    },
    Binding {
        host_key: 'X',
        host_label: "X",
        chip8_index: 0x0,
        chip8_label: "0",
    },
    Binding {
        host_key: 'C',
        host_label: "C",
        chip8_index: 0xB,
        chip8_label: "B",
    },
    Binding {
        host_key: 'V',
        host_label: "V",
        chip8_index: 0xF,
        chip8_label: "F",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Message {
    Press(usize),
    Release(usize),
}

pub(super) fn map_key(key: &Key) -> Option<usize> {
    let Key::Character(character) = key else {
        return None;
    };

    let mut characters = character.chars();
    let host_key = characters.next()?;
    if characters.next().is_some() {
        return None;
    }

    BINDINGS
        .iter()
        .find(|binding| binding.host_key.eq_ignore_ascii_case(&host_key))
        .map(|binding| binding.chip8_index)
}

pub(super) fn view() -> Element<'static, Message> {
    let mut grid = Column::new().spacing(6).width(Length::Fill);

    for bindings in BINDINGS.chunks_exact(4) {
        let mut row = Row::new().spacing(6).width(Length::Fill);
        for binding in bindings {
            row = row.push(key_control(*binding));
        }
        grid = grid.push(row);
    }

    let content = Column::new()
        .push(Text::new("Controller").size(24))
        .push(Text::new("CHIP-8 key / host key").size(14))
        .push(grid)
        .push(Text::new("Control meanings are ROM-specific.").size(14))
        .spacing(8)
        .width(Length::Fill);

    let scrollable_content = Container::new(content).width(Length::Fill).padding([8, 16]);

    Container::new(Scrollable::new(scrollable_content))
        .width(Length::Fixed(SIDEBAR_WIDTH))
        .height(Length::Fill)
        .into()
}

fn key_control(binding: Binding) -> Element<'static, Message> {
    let host_label = Row::new()
        .push(Text::new("Host ").size(12))
        .push(Text::new(binding.host_label).size(12));
    let label = Column::new()
        .push(Text::new(binding.chip8_label).size(22))
        .push(host_label)
        .spacing(2)
        .align_items(Alignment::Center);
    let target = Container::new(label)
        .width(Length::Fill)
        .height(Length::Fixed(KEY_HEIGHT))
        .center_x()
        .center_y()
        .style(iced::theme::Container::Box);

    MouseArea::new(target)
        .on_press(Message::Press(binding.chip8_index))
        .on_release(Message::Release(binding.chip8_index))
        .on_exit(Message::Release(binding.chip8_index))
        .interaction(iced::mouse::Interaction::Pointer)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_are_complete_unique_and_in_canonical_grid_order() {
        assert_eq!(
            BINDINGS.map(|binding| binding.chip8_index),
            [0x1, 0x2, 0x3, 0xC, 0x4, 0x5, 0x6, 0xD, 0x7, 0x8, 0x9, 0xE, 0xA, 0x0, 0xB, 0xF,]
        );
        assert_eq!(
            BINDINGS.map(|binding| binding.chip8_label),
            ["1", "2", "3", "C", "4", "5", "6", "D", "7", "8", "9", "E", "A", "0", "B", "F",]
        );
        assert_eq!(
            BINDINGS.map(|binding| binding.host_key),
            ['1', '2', '3', '4', 'Q', 'W', 'E', 'R', 'A', 'S', 'D', 'F', 'Z', 'X', 'C', 'V',]
        );
        assert_eq!(
            BINDINGS.map(|binding| binding.host_label),
            ["1", "2", "3", "4", "Q", "W", "E", "R", "A", "S", "D", "F", "Z", "X", "C", "V",]
        );

        for (position, binding) in BINDINGS.iter().enumerate() {
            assert!(binding.chip8_index < 16);
            assert!(!BINDINGS[..position]
                .iter()
                .any(|candidate| candidate.chip8_index == binding.chip8_index));
            assert!(!BINDINGS[..position]
                .iter()
                .any(|candidate| candidate.host_key.eq_ignore_ascii_case(&binding.host_key)));
        }
    }
}
