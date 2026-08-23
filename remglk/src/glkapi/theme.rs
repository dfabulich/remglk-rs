/*

Theme table and glk_style_measure support
=========================================

Copyright (c) 2026 Dannii Willis
MIT licenced
https://github.com/curiousdannii/remglk-rs

*/

use super::constants::*;
use super::protocol::{StyleEntry, StyleTable, Theme};

pub type HintMatrix = [[Option<i32>; stylehint_NUMHINTS as usize]; style_NUMSTYLES as usize];

/** Runner-provided theme table used by glk_style_measure */
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeState {
    pub stylehints_enabled: bool,
    buffer: [StyleEntry; style_NUMSTYLES as usize],
    grid: [StyleEntry; style_NUMSTYLES as usize],
}

impl Default for ThemeState {
    fn default() -> Self {
        Self::from_light_defaults(true)
    }
}

impl ThemeState {
    pub fn from_light_defaults(stylehints_enabled: bool) -> Self {
        let buffer_normal = StyleEntry {
            fg: 0x222222,
            bg: 0xFFFFFF,
            weight: 0,
            oblique: 0,
            proportional: 1,
            reverse: 0,
        };
        let grid_normal = StyleEntry {
            fg: 0x000000,
            bg: 0xFFFFFF,
            weight: 0,
            oblique: 0,
            proportional: 0,
            reverse: 0,
        };
        Self {
            stylehints_enabled,
            buffer: [buffer_normal; style_NUMSTYLES as usize],
            grid: [grid_normal; style_NUMSTYLES as usize],
        }
    }

    pub fn apply(&mut self, theme: Theme) {
        self.stylehints_enabled = theme.stylehints_enabled;
        self.buffer = table_to_array(theme.buffer, self.buffer[style_Normal as usize]);
        self.grid = table_to_array(theme.grid, self.grid[style_Normal as usize]);
    }

    pub fn entry(&self, wintype: WindowType, style: u32) -> StyleEntry {
        let table = match wintype {
            WindowType::Buffer => &self.buffer,
            WindowType::Grid => &self.grid,
            _ => return self.buffer[style_Normal as usize],
        };
        if style < style_NUMSTYLES {
            table[style as usize]
        } else {
            table[style_Normal as usize]
        }
    }

    pub fn measure_theme(&self, wintype: WindowType, style: u32, hint: u32) -> Option<u32> {
        if style >= style_NUMSTYLES || hint >= stylehint_NUMHINTS {
            return None;
        }

        #[allow(non_upper_case_globals)]
        match hint {
            stylehint_Indentation | stylehint_ParaIndentation => Some(0),
            stylehint_Justification => Some(stylehint_just_LeftFlush),
            stylehint_Size => Some(1),
            stylehint_Weight => Some(self.entry(wintype, style).weight as u32),
            stylehint_Oblique => Some(self.entry(wintype, style).oblique),
            stylehint_Proportional => Some(self.entry(wintype, style).proportional),
            stylehint_TextColor => Some(self.entry(wintype, style).fg),
            stylehint_BackColor => Some(self.entry(wintype, style).bg),
            stylehint_ReverseColor => Some(self.entry(wintype, style).reverse),
            _ => None,
        }
    }
}

fn table_to_array(table: StyleTable, fallback: StyleEntry) -> [StyleEntry; style_NUMSTYLES as usize] {
    let mut out = [fallback; style_NUMSTYLES as usize];
    for (i, entry) in table.into_iter().enumerate().take(style_NUMSTYLES as usize) {
        out[i] = entry;
    }
    out
}

pub fn measure_style(
    theme: &ThemeState,
    hints_enabled: bool,
    hint_matrix: &HintMatrix,
    wintype: WindowType,
    style: u32,
    hint: u32,
) -> Option<u32> {
    if style >= style_NUMSTYLES || hint >= stylehint_NUMHINTS {
        return None;
    }

    if hints_enabled {
        if let Some(value) = hint_matrix[style as usize][hint as usize] {
            return Some(value as u32);
        }
    }

    theme.measure_theme(wintype, style, hint)
}

pub fn set_hint(hint_matrix: &mut HintMatrix, style: u32, hint: u32, value: i32) {
    if style < style_NUMSTYLES && hint < stylehint_NUMHINTS {
        hint_matrix[style as usize][hint as usize] = Some(value);
    }
}

pub fn clear_hint(hint_matrix: &mut HintMatrix, style: u32, hint: u32) {
    if style < style_NUMSTYLES && hint < stylehint_NUMHINTS {
        hint_matrix[style as usize][hint as usize] = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_hints() -> HintMatrix {
        [[None; stylehint_NUMHINTS as usize]; style_NUMSTYLES as usize]
    }

    fn test_theme() -> ThemeState {
        let mut theme = ThemeState::from_light_defaults(true);
        let mut input = theme.buffer[style_Normal as usize];
        input.fg = 0x0B4C8E;
        theme.buffer[style_Input as usize] = input;
        theme
    }

    #[test]
    fn hint_round_trip_when_enabled() {
        let theme = test_theme();
        let mut hints = empty_hints();
        set_hint(&mut hints, style_Normal, stylehint_TextColor, 0x123456);
        set_hint(&mut hints, style_Normal, stylehint_BackColor, 0x654321);

        assert_eq!(
            measure_style(&theme, true, &hints, WindowType::Buffer, style_Normal, stylehint_TextColor),
            Some(0x123456)
        );
        assert_eq!(
            measure_style(&theme, true, &hints, WindowType::Buffer, style_Normal, stylehint_BackColor),
            Some(0x654321)
        );
    }

    #[test]
    fn theme_wins_when_hints_disabled() {
        let theme = test_theme();
        let mut hints = empty_hints();
        set_hint(&mut hints, style_Normal, stylehint_TextColor, 0x123456);

        assert_eq!(
            measure_style(&theme, false, &hints, WindowType::Buffer, style_Normal, stylehint_TextColor),
            Some(0x222222)
        );
    }

    #[test]
    fn grid_vs_buffer_window_type() {
        let mut theme = ThemeState::from_light_defaults(true);
        theme.grid[style_Normal as usize].fg = 0xDDDDDD;
        theme.grid[style_Normal as usize].bg = 0x111111;
        let hints = empty_hints();

        assert_eq!(
            measure_style(&theme, true, &hints, WindowType::Grid, style_Normal, stylehint_TextColor),
            Some(0xDDDDDD)
        );
        assert_eq!(
            measure_style(&theme, true, &hints, WindowType::Grid, style_Normal, stylehint_BackColor),
            Some(0x111111)
        );
    }

    #[test]
    fn per_style_theme_entry() {
        let theme = test_theme();
        let hints = empty_hints();

        assert_eq!(
            measure_style(&theme, true, &hints, WindowType::Buffer, style_Input, stylehint_TextColor),
            Some(0x0B4C8E)
        );
    }

    #[test]
    fn non_colour_hardcoded_hints() {
        let theme = test_theme();
        let hints = empty_hints();

        assert_eq!(
            measure_style(&theme, true, &hints, WindowType::Buffer, style_Normal, stylehint_Indentation),
            Some(0)
        );
        assert_eq!(
            measure_style(&theme, true, &hints, WindowType::Buffer, style_Normal, stylehint_Justification),
            Some(stylehint_just_LeftFlush)
        );
        assert_eq!(
            measure_style(&theme, true, &hints, WindowType::Buffer, style_Normal, stylehint_Size),
            Some(1)
        );
    }

    #[test]
    fn clear_hint_restores_theme() {
        let theme = test_theme();
        let mut hints = empty_hints();
        set_hint(&mut hints, style_Normal, stylehint_TextColor, 0x123456);
        clear_hint(&mut hints, style_Normal, stylehint_TextColor);

        assert_eq!(
            measure_style(&theme, true, &hints, WindowType::Buffer, style_Normal, stylehint_TextColor),
            Some(0x222222)
        );
    }

    #[test]
    fn apply_theme_from_protocol() {
        let mut theme = ThemeState::default();
        theme.apply(Theme {
            stylehints_enabled: false,
            buffer: vec![StyleEntry {
                fg: 0x111111,
                bg: 0xEEEEEE,
                weight: 0,
                oblique: 0,
                proportional: 1,
                reverse: 0,
            }],
            grid: vec![StyleEntry {
                fg: 0x222222,
                bg: 0x333333,
                weight: 0,
                oblique: 0,
                proportional: 0,
                reverse: 0,
            }],
        });
        assert!(!theme.stylehints_enabled);
        assert_eq!(theme.entry(WindowType::Buffer, style_Normal).fg, 0x111111);
        assert_eq!(theme.entry(WindowType::Grid, style_Normal).bg, 0x333333);
    }
}
