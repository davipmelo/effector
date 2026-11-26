//! Customisable keyboard shortcuts. `cmd` is ⌘ on macOS and Ctrl on Windows; `opt` is ⌥ / Alt.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyCombo {
    /// lowercased character or special name (delete, fdelete, left, right, up, down, space, return, tab, escape)
    pub key: String,
    #[serde(default)]
    pub cmd: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub opt: bool,
    #[serde(default)]
    pub ctrl: bool,
}

pub const IS_MAC: bool = cfg!(target_os = "macos");

impl KeyCombo {
    pub fn new(key: &str, cmd: bool, shift: bool, opt: bool) -> KeyCombo {
        KeyCombo { key: key.into(), cmd, shift, opt, ctrl: false }
    }

    fn key_label(k: &str) -> String {
        let mac = IS_MAC;
        match k {
            "delete" => if mac { "⌫".into() } else { "Backspace".into() },
            "fdelete" => if mac { "⌦".into() } else { "Del".into() },
            "left" => "←".into(),
            "right" => "→".into(),
            "up" => "↑".into(),
            "down" => "↓".into(),
            "space" => "Space".into(),
            "return" => if mac { "↩".into() } else { "Enter".into() },
            "tab" => if mac { "⇥".into() } else { "Tab".into() },
            "escape" => if mac { "⎋".into() } else { "Esc".into() },
            f if f.len() >= 2 && f.starts_with('f') && f[1..].chars().all(|c| c.is_ascii_digit()) => f.to_uppercase(),
            _ => k.to_uppercase(),
        }
    }

    pub fn display(&self) -> String {
        if IS_MAC {
            format!(
                "{}{}{}{}{}",
                if self.ctrl { "⌃" } else { "" },
                if self.opt { "⌥" } else { "" },
                if self.shift { "⇧" } else { "" },
                if self.cmd { "⌘" } else { "" },
                Self::key_label(&self.key)
            )
        } else {
            let mut parts: Vec<String> = vec![];
            if self.cmd || self.ctrl {
                parts.push("Ctrl".into());
            }
            if self.opt {
                parts.push("Alt".into());
            }
            if self.shift {
                parts.push("Shift".into());
            }
            parts.push(Self::key_label(&self.key));
            parts.join("+")
        }
    }

    pub fn has_modifier(&self) -> bool { self.cmd || self.ctrl }

    pub fn key_name(k: egui::Key) -> Option<&'static str> {
        use egui::Key::*;
        Some(match k {
            Backspace => "delete",
            Delete => "fdelete",
            ArrowLeft => "left",
            ArrowRight => "right",
            ArrowUp => "up",
            ArrowDown => "down",
            Space => "space",
            Enter => "return",
            Tab => "tab",
            Escape => "escape",
            F1 => "f1",
            F2 => "f2",
            F3 => "f3",
            F4 => "f4",
            F5 => "f5",
            F6 => "f6",
            F7 => "f7",
            F8 => "f8",
            F9 => "f9",
            F10 => "f10",
            F11 => "f11",
            F12 => "f12",
            Backslash => "\\",
            Slash => "/",
            OpenBracket => "[",
            CloseBracket => "]",
            Equals | Plus => "=",
            Minus => "-",
            Comma => ",",
            Period => ".",
            Semicolon => ";",
            Quote => "'",
            Backtick => "`",
            Num0 => "0",
            Num1 => "1",
            Num2 => "2",
            Num3 => "3",
            Num4 => "4",
            Num5 => "5",
            Num6 => "6",
            Num7 => "7",
            Num8 => "8",
            Num9 => "9",
            A => "a",
            B => "b",
            C => "c",
            D => "d",
            E => "e",
            F => "f",
            G => "g",
            H => "h",
            I => "i",
            J => "j",
            K => "k",
            L => "l",
            M => "m",
            N => "n",
            O => "o",
            P => "p",
            Q => "q",
            R => "r",
            S => "s",
            T => "t",
            U => "u",
            V => "v",
            W => "w",
            X => "x",
            Y => "y",
            Z => "z",
            _ => return None,
        })
    }

    pub fn from_event(key: egui::Key, m: egui::Modifiers) -> Option<KeyCombo> {
        let k = Self::key_name(key)?;
        Some(KeyCombo {
            key: k.into(),
            cmd: m.command,
            shift: m.shift,
            opt: m.alt,
            // on Windows/Linux `command` already is Ctrl
            ctrl: IS_MAC && m.ctrl,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Open, Save, SaveAs, Export, CopyResult, Paste, Undo, Redo,
    CommandPalette, AddEffect, BeforeAfter, Crop,
    Group, Ungroup, Duplicate, Delete, ToggleVisibility, MoveUp, MoveDown,
    PrevImage, NextImage, Filmstrip, SyncSettings, CloseImage,
    ZoomIn, ZoomOut, ZoomFit, Zoom100, Present, Compare, Randomize, CopyLayers, PasteLayers, ShowLayers, ShowPresets,
    CopySettings, PasteSettings, ResetSettings, TogglePanels, ToggleAllPanels, SelectAll, Deselect,
    ShowClipping, ToggleHistogram, ToolEdit, ToolCanvas, ToolMasks, MaskBrush, MaskLinear, MaskRadial, MaskSubject, MaskBackground,
}

impl Action {
    pub const ALL: [Action; 52] = [
        Action::Open, Action::Save, Action::SaveAs, Action::Export, Action::CopyResult, Action::Paste, Action::Undo, Action::Redo,
        Action::CommandPalette, Action::AddEffect, Action::BeforeAfter, Action::Crop, Action::Group, Action::Ungroup, Action::Duplicate,
        Action::Delete, Action::ToggleVisibility, Action::MoveUp, Action::MoveDown, Action::PrevImage, Action::NextImage,
        Action::Filmstrip, Action::SyncSettings, Action::CloseImage, Action::ZoomIn, Action::ZoomOut, Action::ZoomFit, Action::Zoom100,
        Action::Present, Action::Compare, Action::Randomize, Action::CopyLayers, Action::PasteLayers, Action::ShowLayers,
        Action::ShowPresets, Action::CopySettings, Action::PasteSettings, Action::ResetSettings, Action::TogglePanels,
        Action::ToggleAllPanels, Action::SelectAll, Action::Deselect, Action::ShowClipping, Action::ToggleHistogram,
        Action::ToolEdit, Action::ToolCanvas, Action::ToolMasks, Action::MaskBrush, Action::MaskLinear, Action::MaskRadial,
        Action::MaskSubject, Action::MaskBackground,
    ];

    /// Stable id (same raw values as the macOS app).
    pub fn id(&self) -> &'static str {
        use Action::*;
        match self {
            Open => "open", Save => "save", SaveAs => "saveAs", Export => "export", CopyResult => "copyResult", Paste => "paste",
            Undo => "undo", Redo => "redo", CommandPalette => "commandPalette", AddEffect => "addEffect", BeforeAfter => "beforeAfter",
            Crop => "crop", Group => "group", Ungroup => "ungroup", Duplicate => "duplicate", Delete => "delete",
            ToggleVisibility => "toggleVisibility", MoveUp => "moveUp", MoveDown => "moveDown", PrevImage => "prevImage",
            NextImage => "nextImage", Filmstrip => "filmstrip", SyncSettings => "syncSettings", CloseImage => "closeImage",
            ZoomIn => "zoomIn", ZoomOut => "zoomOut", ZoomFit => "zoomFit", Zoom100 => "zoom100", Present => "present",
            Compare => "compare", Randomize => "randomize", CopyLayers => "copyLayers", PasteLayers => "pasteLayers",
            ShowLayers => "showLayers", ShowPresets => "showPresets", CopySettings => "copySettings",
            PasteSettings => "pasteSettings", ResetSettings => "resetSettings", TogglePanels => "togglePanels",
            ToggleAllPanels => "toggleAllPanels", SelectAll => "selectAll", Deselect => "deselect",
            ShowClipping => "showClipping", ToggleHistogram => "toggleHistogram", ToolEdit => "toolEdit",
            ToolCanvas => "toolCanvas", ToolMasks => "toolMasks", MaskBrush => "maskBrush", MaskLinear => "maskLinear",
            MaskRadial => "maskRadial", MaskSubject => "maskSubject", MaskBackground => "maskBackground",
        }
    }

    pub fn label(&self) -> &'static str {
        use Action::*;
        match self {
            Open => "Open…",
            Save => "Save Project",
            SaveAs => "Save Project As…",
            Export => "Export Image…",
            CopyResult => "Copy Result",
            Paste => "Paste Image",
            Undo => "Undo",
            Redo => "Redo",
            CommandPalette => "Command Palette",
            AddEffect => "Add Effect…",
            BeforeAfter => "Toggle Before / After",
            Crop => "Crop",
            Group => "Group Layer",
            Ungroup => "Ungroup",
            Duplicate => "Duplicate Layer",
            Delete => "Delete Layer",
            ToggleVisibility => "Show / Hide Layer",
            MoveUp => "Move Layer Up",
            MoveDown => "Move Layer Down",
            ZoomIn => "Zoom In",
            ZoomOut => "Zoom Out",
            ZoomFit => "Fit to Window",
            Zoom100 => "Actual Size (100%)",
            Present => "Full Screen Preview",
            PrevImage => "Previous Image",
            NextImage => "Next Image",
            Filmstrip => "Show / Hide Filmstrip",
            SyncSettings => "Sync Settings to Selected Images",
            CloseImage => "Close Image",
            Compare => "Split Before / After",
            Randomize => "Randomize Layer Settings",
            CopyLayers => "Copy Layers",
            PasteLayers => "Paste Layers",
            ShowLayers => "Close Panels",
            ShowPresets => "Presets…",
            CopySettings => "Copy Settings",
            PasteSettings => "Paste Settings",
            ResetSettings => "Reset Settings",
            TogglePanels => "Show / Hide Side Panels",
            ToggleAllPanels => "Show / Hide All Panels",
            SelectAll => "Select All",
            Deselect => "Deselect All",
            ShowClipping => "Show Clipping",
            ToggleHistogram => "Show / Hide Histogram",
            ToolEdit => "Edit Tool",
            ToolCanvas => "Canvas & Borders",
            ToolMasks => "Masks",
            MaskBrush => "Mask: Brush",
            MaskLinear => "Mask: Linear Gradient",
            MaskRadial => "Mask: Radial Gradient",
            MaskSubject => "Mask: Select Subject",
            MaskBackground => "Mask: Select Background",
        }
    }

    pub fn default_combo(&self) -> Option<KeyCombo> {
        use Action::*;
        let k = |key: &str, cmd: bool, shift: bool, opt: bool| Some(KeyCombo::new(key, cmd, shift, opt));
        // Lightroom Classic where it has the same function, Photoshop for layer commands.
        match self {
            Open => k("o", true, false, false),
            Save => k("s", true, false, false),
            SaveAs => k("s", true, false, true),
            Export => k("e", true, true, false),
            CopyResult => k("c", true, true, true),
            Paste => k("v", true, false, false),
            Undo => k("z", true, false, false),
            Redo => k("z", true, true, false),
            CommandPalette => k("k", true, false, false),
            AddEffect => k("a", true, true, false),
            BeforeAfter => k("]", false, false, false),
            Crop => k("r", false, false, false),
            Group => k("g", true, false, false),
            Ungroup => k("g", true, true, false),
            Duplicate => k("j", true, false, false),
            Delete => k("delete", false, false, false),
            ToggleVisibility => k("h", false, false, false),
            MoveUp => k("]", true, false, false),
            MoveDown => k("[", true, false, false),
            ZoomIn => k("=", true, false, false),
            ZoomOut => k("-", true, false, false),
            ZoomFit => k("z", false, false, false),
            Zoom100 => k("1", true, false, false),
            Present => k("f", false, false, false),
            PrevImage => k("left", false, false, false),
            NextImage => k("right", false, false, false),
            Filmstrip => k("f6", false, false, false),
            SyncSettings => k("s", true, true, false),
            CloseImage => k("w", true, false, false),
            Compare => k("y", false, false, false),
            Randomize => k("r", false, true, false),
            CopyLayers => k("c", true, false, true),
            PasteLayers => k("v", true, false, true),
            ShowLayers => k("l", false, false, false),
            ShowPresets => k("p", false, false, false),
            CopySettings => k("c", true, true, false),
            PasteSettings => k("v", true, true, false),
            ResetSettings => k("r", true, true, false),
            TogglePanels => k("tab", false, false, false),
            ToggleAllPanels => k("tab", false, true, false),
            SelectAll => k("a", true, false, false),
            Deselect => k("d", true, false, false),
            ShowClipping => k("j", false, false, false),
            ToggleHistogram => k("h", true, true, false),
            ToolEdit => k("e", false, false, false),
            ToolCanvas => k("b", false, true, false),
            ToolMasks => k("w", false, true, false),
            // Lightroom Classic: K brush, M linear, ⇧M radial (subject / background have no LR default)
            MaskBrush => k("k", false, false, false),
            MaskLinear => k("m", false, false, false),
            MaskRadial => k("m", false, true, false),
            MaskSubject => k("s", false, true, false),
            MaskBackground => k("g", false, true, false),
        }
    }

    /// Standard text-editing shortcuts that must reach text fields while typing.
    pub fn is_text_editing_standard(&self) -> bool {
        matches!(self, Action::Undo | Action::Redo | Action::Paste | Action::CopyResult | Action::Delete | Action::SelectAll | Action::Deselect)
    }
}

/// Current combo for an action (custom or default). Custom value "none" = disabled.
pub fn combo(a: Action) -> Option<KeyCombo> {
    let s = crate::settings::get();
    match s.shortcuts.get(a.id()) {
        Some(v) if v == "none" => None,
        Some(v) => serde_json::from_str(v).ok().or_else(|| a.default_combo()),
        None => a.default_combo(),
    }
}

pub fn display(a: Action) -> String { combo(a).map(|c| c.display()).unwrap_or_default() }

/// Assigns a combo, removing it from any other action to avoid conflicts.
pub fn set(a: Action, c: Option<KeyCombo>) {
    let others: Vec<Action> = Action::ALL.iter().copied().filter(|o| *o != a && c.is_some() && combo(*o) == c).collect();
    crate::settings::update(|s| {
        for o in others {
            s.shortcuts.insert(o.id().into(), "none".into());
        }
        s.shortcuts.insert(a.id().into(), c.map(|c| serde_json::to_string(&c).unwrap()).unwrap_or_else(|| "none".into()));
    });
}

pub fn reset_all() { crate::settings::update(|s| s.shortcuts.clear()); }

pub fn action_for(c: &KeyCombo) -> Option<Action> { Action::ALL.iter().copied().find(|a| combo(*a).as_ref() == Some(c)) }
