//! Application state and actions (the equivalent of the macOS app's AppState): open images
//! (filmstrip), editing with undo, layer operations, preview rendering, sessions, projects, export.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::effects::{self, Look};
use crate::gpu::T;
use crate::imageio::{self, Format, SourceImage};
use crate::l10n::{tr, trf};
use crate::model::*;
use crate::presets::{self, Preset};
use crate::settings;
use crate::shortcuts::Action;
use crate::worker::{RenderMsg, RenderRequest, RenderWorker, ThumbJob, ThumbResult, ThumbWorker};

/// One open image in the workspace: its original file, its edits and its undo history.
pub struct DocSlot {
    pub id: Uuid,
    pub data: Arc<Vec<u8>>,
    pub file_name: String,
    pub file_path: Option<PathBuf>,
    /// Decoded pixels (released for images not used recently).
    pub source: Option<Arc<SourceImage>>,
    pub state: DocumentState,
    pub undo: Vec<DocumentState>,
    pub redo: Vec<DocumentState>,
    pub project_path: Option<PathBuf>,
    pub is_dirty: bool,
    pub selected_id: Option<Uuid>,
    pub zoom: Option<f64>,
    pub pan: egui::Vec2,
    pub last_used: Instant,
    /// File name inside the session folder.
    pub session_file: Option<String>,
    pub thumb: Option<egui::TextureHandle>,
}

impl DocSlot {
    fn new(data: Arc<Vec<u8>>, file_name: &str, file_path: Option<PathBuf>, source: Option<Arc<SourceImage>>) -> DocSlot {
        DocSlot {
            id: Uuid::new_v4(),
            data,
            file_name: file_name.into(),
            file_path,
            source,
            state: DocumentState::default(),
            undo: vec![],
            redo: vec![],
            project_path: None,
            is_dirty: false,
            selected_id: None,
            zoom: None,
            pan: egui::Vec2::ZERO,
            last_used: Instant::now(),
            session_file: None,
            thumb: None,
        }
    }

    pub fn load_source(&mut self) -> Result<Arc<SourceImage>, String> {
        if let Some(s) = &self.source {
            return Ok(s.clone());
        }
        let s = Arc::new(SourceImage::load_data(self.data.clone(), &self.file_name)?);
        self.source = Some(s.clone());
        Ok(s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropPosition {
    Above,
    Below,
    Inside,
}

/// Tools of the right sidebar (the icon row under the histogram), Lightroom-style.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Tool {
    /// The selected layer's settings (or the image info).
    #[default]
    Edit,
    Crop,
    /// Canvas & Borders.
    Canvas,
    Masks,
}

/// Drawing tools of the Masks panel (Lightroom Classic: K, M, ⇧M).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaskDraw {
    Brush,
    Linear,
    Radial,
}

/// Brush settings (the next strokes use them). Size is a fraction of the photo's long side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushSettings {
    pub size: f32,
    pub feather: f32,
    pub flow: f32,
}

impl Default for BrushSettings {
    fn default() -> Self { BrushSettings { size: 0.035, feather: 0.6, flow: 0.8 } }
}

#[derive(Clone, Copy, Default)]
pub struct SyncOptions {
    pub layers: bool,
    pub crop: bool,
    pub canvas: bool,
    pub add_on_top: bool,
}

/// Modal dialogs the UI shows on behalf of actions.
pub enum Dialog {
    Alert { title: String, message: String },
    Prompt { title: String, label: String, text: String, kind: PromptKind },
    ConfirmClose { ids: HashSet<Uuid> },
}

pub enum PromptKind {
    SaveStackPreset,
    SaveLayersPreset(Vec<LayerNode>),
    RenamePreset(Uuid),
    SaveGradient { layer: Uuid, key: String, gradient: GradientSpec },
    SavePalette { layer: Uuid, key: String, palette: PaletteSpec },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionDoc {
    file_name: String,
    original_file: String,
    state: DocumentState,
    #[serde(default)]
    project_path: Option<String>,
    #[serde(default)]
    dirty: bool,
    #[serde(default)]
    file_path: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct SessionFile {
    #[serde(default = "two")]
    version: i64,
    active: usize,
    docs: Vec<SessionDoc>,
}
fn two() -> i64 { 2 }

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionFileV1 {
    file_name: String,
    original_file: String,
    state: DocumentState,
    #[serde(default)]
    project_path: Option<String>,
    #[serde(default)]
    dirty: bool,
}

pub struct AppState {
    pub docs: Vec<DocSlot>,
    pub active_id: Option<Uuid>,
    /// Images selected in the filmstrip (for sync / batch export).
    pub strip_selection: HashSet<Uuid>,
    pub show_sync: bool,

    /// Primary selection (shown in the inspector).
    pub selected_id: Option<Uuid>,
    /// All selected layers (shift / cmd-click).
    pub selected_ids: HashSet<Uuid>,
    selection_anchor: Option<Uuid>,
    /// Temporary layers drawn on top while hovering / browsing effects and presets.
    pub preview_layers: Option<Vec<LayerNode>>,
    hover_pending: Option<(Option<Vec<LayerNode>>, Instant)>,
    /// Bumped when a real edit drops the hover preview (see `state_changed`).
    pub hover_epoch: u64,
    /// `textures::generation()` seen last (the library arrived: render and thumbnails again).
    tex_gen: u64,

    // View
    pub show_original: bool,
    pub compare_split: bool,
    pub split_pos: f64,
    pub presenting: bool,
    /// Tab hides the side panels, Shift+Tab every panel (Lightroom).
    pub hide_side_panels: bool,
    /// Lightroom-style clipping warnings on the canvas (J).
    pub show_clipping: bool,
    /// Histogram of the current result (None until measured, or while the panel is closed).
    pub histogram: Option<Arc<crate::histogram::Histogram>>,
    /// The update installer was started: quit now (the UI closes the window).
    pub quit_for_update: bool,
    update_announced: bool,
    /// On-screen size the last display copy was made for (a new zoom makes a new copy).
    last_display_target: Option<(usize, usize)>,
    /// The launch check is running: when it finds a new version, the update pop-up opens.
    pub update_check_auto: bool,
    /// The "update available" pop-up is open.
    pub show_update_popup: bool,
    subject_gen: u64,
    subject_missing_warned: bool,
    /// Masks tool: drawing a new linear / radial gradient or painting with the brush.
    pub mask_draw: Option<MaskDraw>,
    /// Where the next drawn part goes: an existing mask (with a mode) or a new mask (None).
    pub mask_draw_into: Option<(Uuid, MaskMode)>,
    /// The part of the selected mask whose handles show on the image.
    pub selected_part: Option<Uuid>,
    pub brush: BrushSettings,
    /// Brush erases (the Erase button; holding ⌥/Alt also erases).
    pub brush_erase: bool,
    pub hide_all_panels: bool,
    pub fit_zoom: f64,
    pub viewport: egui::Vec2,
    pub pixels_per_point: f64,
    pub output: Option<T>,
    pub original: Option<T>,
    pub display: Option<T>,
    pub display_original: Option<T>,
    pub crop_view: Option<T>,
    /// Red overlay of the selected mask (Masks tool).
    pub mask_view: Option<T>,
    pub display_gen: u64,
    pub is_rendering: bool,
    pub is_cropping: bool,
    pub crop_draft: CropRect,
    pub crop_aspect: String,
    pub show_effect_browser: bool,
    pub show_preset_browser: bool,
    pub show_command_palette: bool,
    pub show_export: bool,
    pub show_settings: bool,
    /// Active tool of the right sidebar.
    pub tool: Tool,
    /// Mask selected in the Masks tool.
    pub selected_mask: Option<Uuid>,
    /// Layer that a mask created next in the Masks tool is applied to (set from the layer's mask slot).
    pub mask_new_for: Option<Uuid>,
    pub is_exporting: bool,
    pub export_progress: Option<String>,
    /// Files done / total of the running export.
    pub export_counter: Option<(Arc<std::sync::atomic::AtomicUsize>, usize)>,
    pub effect_thumbs: HashMap<String, egui::TextureHandle>,
    pub preset_thumbs: HashMap<Uuid, egui::TextureHandle>,
    pub toast: Option<(String, Instant)>,
    pub dialog: Option<Dialog>,
    copied_settings: Option<DocumentState>,

    // Undo
    last_coalesce_key: Option<String>,
    last_coalesce_time: Instant,
    browsing: bool,
    browse_undo_pushed: bool,

    // Rendering
    render: RenderWorker,
    thumbs: ThumbWorker,
    render_pending: bool,
    render_running: bool,
    /// When the in-flight render was sent (watchdog).
    render_started: Instant,
    render_gen: u64,
    pub preview_scale: f64,
    interactive_until: Instant,
    refine_at: Option<Instant>,
    effects_gen: u64,
    presets_gen: u64,
    thumb_due: Option<Instant>,
    session_due: Option<Instant>,
    restoring: bool,
    pub headless: bool,
    ctx: egui::Context,
    export_rx: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    copy_rx: Option<std::sync::mpsc::Receiver<Option<(Vec<u8>, usize, usize)>>>,
}

pub fn now() -> Instant { Instant::now() }

impl AppState {
    pub fn new(ctx: egui::Context) -> AppState {
        let s = settings::get();
        AppState {
            docs: vec![],
            active_id: None,
            strip_selection: HashSet::new(),
            show_sync: false,
            selected_id: None,
            selected_ids: HashSet::new(),
            selection_anchor: None,
            preview_layers: None,
            hover_pending: None,
            hover_epoch: 0,
            tex_gen: 0,
            show_original: false,
            compare_split: false,
            split_pos: 0.5,
            presenting: false,
            hide_side_panels: false,
            show_clipping: false,
            histogram: None,
            quit_for_update: false,
            update_announced: false,
            last_display_target: None,
            update_check_auto: false,
            show_update_popup: false,
            subject_gen: 0,
            subject_missing_warned: false,
            mask_draw: None,
            mask_draw_into: None,
            selected_part: None,
            brush: BrushSettings::default(),
            brush_erase: false,
            hide_all_panels: false,
            fit_zoom: 1.0,
            viewport: egui::Vec2::ZERO,
            pixels_per_point: 1.0,
            output: None,
            original: None,
            display: None,
            display_original: None,
            crop_view: None,
            mask_view: None,
            display_gen: 0,
            is_rendering: false,
            is_cropping: false,
            crop_draft: CropRect::default(),
            crop_aspect: "free".into(),
            show_effect_browser: false,
            show_preset_browser: s.presets_open,
            show_command_palette: false,
            show_export: false,
            show_settings: false,
            tool: Tool::Edit,
            selected_mask: None,
            mask_new_for: None,
            is_exporting: false,
            export_progress: None,
            export_counter: None,
            effect_thumbs: HashMap::new(),
            preset_thumbs: HashMap::new(),
            toast: None,
            dialog: None,
            copied_settings: None,
            last_coalesce_key: None,
            last_coalesce_time: Instant::now() - Duration::from_secs(10),
            browsing: false,
            browse_undo_pushed: false,
            render: { crate::textures::set_notify(ctx.clone()); RenderWorker::start(ctx.clone()) },
            thumbs: ThumbWorker::start(ctx.clone()),
            render_pending: false,
            render_running: false,
            render_started: Instant::now(),
            render_gen: 0,
            preview_scale: 0.25,
            interactive_until: Instant::now(),
            refine_at: None,
            effects_gen: 0,
            presets_gen: 0,
            thumb_due: None,
            session_due: None,
            restoring: false,
            headless: false,
            ctx,
            export_rx: None,
            copy_rx: None,
        }
    }

    // MARK: - Active document

    pub fn active_index(&self) -> Option<usize> { self.docs.iter().position(|d| Some(d.id) == self.active_id) }
    pub fn doc(&self) -> Option<&DocSlot> { self.active_index().map(|i| &self.docs[i]) }
    pub fn doc_mut(&mut self) -> Option<&mut DocSlot> { self.active_index().map(move |i| &mut self.docs[i]) }
    pub fn has_image(&self) -> bool { self.doc().and_then(|d| d.source.as_ref()).is_some() }
    pub fn source(&self) -> Option<Arc<SourceImage>> { self.doc().and_then(|d| d.source.clone()) }
    pub fn state(&self) -> DocumentState { self.doc().map(|d| d.state.clone()).unwrap_or_default() }
    pub fn layers(&self) -> &[LayerNode] { self.doc().map(|d| d.state.layers.as_slice()).unwrap_or(&[]) }
    pub fn is_dirty(&self) -> bool { self.doc().map(|d| d.is_dirty).unwrap_or(false) }
    pub fn project_path(&self) -> Option<PathBuf> { self.doc().and_then(|d| d.project_path.clone()) }
    pub fn can_undo(&self) -> bool { self.doc().map(|d| !d.undo.is_empty()).unwrap_or(false) }
    pub fn can_redo(&self) -> bool { self.doc().map(|d| !d.redo.is_empty()).unwrap_or(false) }
    pub fn zoom(&self) -> Option<f64> { self.doc().and_then(|d| d.zoom) }
    pub fn pan(&self) -> egui::Vec2 { self.doc().map(|d| d.pan).unwrap_or_default() }
    pub fn set_pan(&mut self, p: egui::Vec2) {
        if let Some(d) = self.doc_mut() {
            d.pan = p;
        }
    }

    fn sessions_enabled(&self) -> bool { !self.headless && settings::get().reopen_last_session }

    /// Adds an image as a new document and makes it active.
    pub fn open_image(&mut self, img: SourceImage, path: Option<PathBuf>, initial: DocumentState, project: Option<PathBuf>, activate: bool) -> Uuid {
        let img = Arc::new(img);
        let (data, name) = (img.data.clone(), img.file_name.clone());
        let mut slot = DocSlot::new(data, &name, path, Some(img));
        slot.state = initial;
        slot.project_path = project;
        let id = slot.id;
        if !self.restoring && self.sessions_enabled() {
            self.write_session_original(&mut slot);
        }
        self.docs.push(slot);
        if activate {
            self.activate(id);
        }
        self.make_strip_thumb(id);
        self.schedule_session_save();
        id
    }

    /// Switches the editor to another image.
    pub fn activate(&mut self, id: Uuid) {
        let Some(i) = self.docs.iter().position(|d| d.id == id) else { return };
        if Some(id) == self.active_id && self.docs[i].source.is_some() {
            return;
        }
        if self.is_cropping {
            self.cancel_crop();
        }
        self.stash_selection();
        if let Err(e) = self.docs[i].load_source() {
            self.alert(&tr("Could not open file"), &e);
            return;
        }
        self.active_id = Some(id);
        self.docs[i].last_used = Instant::now();
        self.preview_layers = None;
        self.last_coalesce_key = None;
        let sel = self.docs[i].selected_id.filter(|s| self.docs[i].state.layers.find(*s).is_some());
        self.set_selected(sel);
        self.output = None;
        self.display = None;
        self.update_preview_scale(true);
        self.request_render();
        self.regenerate_thumbnails();
        self.unload_inactive_sources();
        if !self.strip_selection.contains(&id) {
            self.strip_selection = [id].into_iter().collect();
        }
        self.schedule_session_save();
    }

    fn stash_selection(&mut self) {
        let sel = self.selected_id;
        if let Some(d) = self.doc_mut() {
            d.selected_id = sel;
        }
    }

    /// Keeps decoded pixels only for the active image and the two most recently used ones.
    fn unload_inactive_sources(&mut self) {
        let mut loaded: Vec<(Instant, Uuid)> = self.docs.iter().filter(|d| d.source.is_some()).map(|d| (d.last_used, d.id)).collect();
        loaded.sort_by(|a, b| b.0.cmp(&a.0));
        let keep: HashSet<Uuid> = loaded.iter().take(3).map(|x| x.1).collect();
        let active = self.active_id;
        for d in self.docs.iter_mut() {
            if Some(d.id) != active && !keep.contains(&d.id) {
                d.source = None;
            }
        }
    }

    pub fn activate_next(&mut self, delta: i64) {
        let Some(i) = self.active_index() else { return };
        let n = (i as i64 + delta).clamp(0, self.docs.len() as i64 - 1) as usize;
        if n != i {
            let id = self.docs[n].id;
            self.activate(id);
        }
    }

    /// Asks before closing images that have effects.
    pub fn request_close(&mut self, ids: HashSet<Uuid>) {
        if ids.is_empty() {
            return;
        }
        let edited = self.docs.iter().any(|d| ids.contains(&d.id) && !d.state.layers.is_empty());
        if edited {
            self.dialog = Some(Dialog::ConfirmClose { ids });
        } else {
            self.close_documents(&ids);
        }
    }

    /// Closes images (their edits are lost unless saved as a project).
    pub fn close_documents(&mut self, ids: &HashSet<Uuid>) {
        let was_active = self.active_id.map(|a| ids.contains(&a)).unwrap_or(false);
        let old_index = self.active_index().unwrap_or(0);
        let files: Vec<String> = self.docs.iter().filter(|d| ids.contains(&d.id)).filter_map(|d| d.session_file.clone()).collect();
        for f in files {
            let _ = std::fs::remove_file(session_dir().join(f));
        }
        self.docs.retain(|d| !ids.contains(&d.id));
        self.strip_selection.retain(|x| !ids.contains(x));
        if was_active {
            self.active_id = None;
            self.set_selected(None);
            self.output = None;
            self.original = None;
            self.display = None;
            self.display_original = None;
            if !self.docs.is_empty() {
                let id = self.docs[old_index.min(self.docs.len() - 1)].id;
                self.activate(id);
            }
        }
        self.schedule_session_save();
    }

    // MARK: Filmstrip helpers

    /// Thumbnail of an image with its own effects (for the filmstrip).
    pub fn make_strip_thumb(&mut self, id: Uuid) {
        let Some(d) = self.docs.iter().find(|d| d.id == id) else { return };
        self.thumbs.send(ThumbJob::Strip {
            doc: id,
            source: d.source.clone(),
            data: d.data.clone(),
            file_name: d.file_name.clone(),
            state: d.state.clone(),
        });
    }

    /// Filmstrip click: plain = open, cmd = toggle in selection, shift = range.
    pub fn strip_click(&mut self, id: Uuid, m: egui::Modifiers) {
        if m.command {
            if self.strip_selection.contains(&id) && self.strip_selection.len() > 1 {
                self.strip_selection.remove(&id);
            } else {
                self.strip_selection.insert(id);
            }
            return;
        }
        if m.shift {
            if let (Some(a), Some(b)) = (self.active_index(), self.docs.iter().position(|d| d.id == id)) {
                self.strip_selection = self.docs[a.min(b)..=a.max(b)].iter().map(|d| d.id).collect();
                return;
            }
        }
        self.strip_selection = [id].into_iter().collect();
        self.activate(id);
    }

    /// Copies the active image's settings to other images (each gets its own undo step).
    pub fn sync_settings(&mut self, ids: &HashSet<Uuid>, opt: SyncOptions) {
        let st = self.state();
        let active = self.active_id;
        let mut n = 0;
        let mut touched = vec![];
        for t in self.docs.iter_mut().filter(|d| ids.contains(&d.id) && Some(d.id) != active) {
            t.undo.push(t.state.clone());
            t.redo.clear();
            if opt.layers {
                let copy: Vec<LayerNode> = st.portable(&st.layers).iter().map(|l| l.reidentified()).collect();
                t.state.layers = if opt.add_on_top {
                    let mut c = copy;
                    c.extend(t.state.layers.drain(..));
                    c
                } else {
                    copy
                };
            }
            if opt.crop {
                t.state.crop = st.crop;
            }
            if opt.canvas {
                t.state.canvas = st.canvas.clone();
            }
            t.state.adopt_masks();
            t.is_dirty = true;
            touched.push(t.id);
            n += 1;
        }
        if n == 0 {
            return;
        }
        for id in touched {
            self.make_strip_thumb(id);
        }
        self.flash(&trf("Settings synced to %d images", &[&n]));
        self.schedule_session_save();
    }

    pub fn has_copied_settings(&self) -> bool { self.copied_settings.is_some() }

    pub fn copy_settings(&mut self) {
        self.copied_settings = Some(self.state());
        self.flash(&tr("Settings copied"));
    }

    /// Pastes copied settings (layers, crop and canvas) onto the selected images.
    pub fn paste_settings(&mut self, ids: &HashSet<Uuid>) {
        let Some(c) = self.copied_settings.clone() else { return };
        let active = self.active_id;
        let mut touched = vec![];
        for t in self.docs.iter_mut().filter(|d| ids.contains(&d.id) && Some(d.id) != active) {
            t.undo.push(t.state.clone());
            t.redo.clear();
            t.state.layers = c.portable(&c.layers).iter().map(|l| l.reidentified()).collect();
            t.state.crop = c.crop;
            t.state.canvas = c.canvas.clone();
            t.state.adopt_masks();
            t.is_dirty = true;
            touched.push(t.id);
        }
        if active.map(|a| ids.contains(&a)).unwrap_or(false) {
            self.edit(None, |s| {
                s.layers = c.portable(&c.layers).iter().map(|l| l.reidentified()).collect();
                s.crop = c.crop;
                s.canvas = c.canvas.clone();
            });
        }
        for id in touched {
            self.make_strip_thumb(id);
        }
        self.flash(&trf("Settings pasted to %d images", &[&ids.len()]));
        self.schedule_session_save();
    }

    pub fn reset_settings(&mut self, ids: &HashSet<Uuid>) {
        let active = self.active_id;
        let mut touched = vec![];
        for t in self.docs.iter_mut().filter(|d| ids.contains(&d.id) && Some(d.id) != active) {
            t.undo.push(t.state.clone());
            t.redo.clear();
            t.state = DocumentState::default();
            t.is_dirty = true;
            touched.push(t.id);
        }
        if active.map(|a| ids.contains(&a)).unwrap_or(false) {
            self.edit(None, |s| *s = DocumentState::default());
        }
        for id in touched {
            self.make_strip_thumb(id);
        }
        self.schedule_session_save();
    }

    // MARK: - Session (reopened on next launch)

    fn write_session_original(&self, slot: &mut DocSlot) {
        let ext = Path::new(&slot.file_name).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_else(|| "png".into());
        let name = format!("img-{}.{}", slot.id.to_string().to_uppercase(), ext);
        slot.session_file = Some(name.clone());
        let data = slot.data.clone();
        std::thread::spawn(move || settings::write_atomic(&session_dir().join(name), &data));
    }

    pub fn schedule_session_save(&mut self) {
        if self.restoring || !self.sessions_enabled() {
            return;
        }
        self.session_due = Some(Instant::now() + Duration::from_millis(800));
    }

    pub fn save_session(&mut self, sync: bool) {
        if self.restoring || !self.sessions_enabled() {
            return;
        }
        let list: Vec<SessionDoc> = self
            .docs
            .iter()
            .filter_map(|d| {
                Some(SessionDoc {
                    file_name: d.file_name.clone(),
                    original_file: d.session_file.clone()?,
                    state: d.state.clone(),
                    project_path: d.project_path.as_ref().map(|p| p.to_string_lossy().to_string()),
                    dirty: d.is_dirty,
                    file_path: d.file_path.as_ref().map(|p| p.to_string_lossy().to_string()),
                })
            })
            .collect();
        let keep: HashSet<String> = list.iter().map(|d| d.original_file.clone()).chain(["session.json".to_string()]).collect();
        let file = SessionFile { version: 2, active: self.active_index().unwrap_or(0), docs: list };
        let data = serde_json::to_vec(&file).unwrap_or_default();
        let work = move || {
            let dir = session_dir();
            settings::write_atomic(&dir.join("session.json"), &data);
            // remove images that are no longer open
            if let Ok(rd) = std::fs::read_dir(&dir) {
                for e in rd.flatten() {
                    let n = e.file_name().to_string_lossy().to_string();
                    if !keep.contains(&n) && !n.ends_with(".tmp~") {
                        let _ = std::fs::remove_file(e.path());
                    }
                }
            }
        };
        if sync {
            work();
        } else {
            std::thread::spawn(work);
        }
    }

    /// Reopens the images and layers that were open when the app last quit.
    pub fn restore_session(&mut self) -> bool {
        if !self.sessions_enabled() || self.has_image() {
            return false;
        }
        let dir = session_dir();
        let Ok(d) = std::fs::read(dir.join("session.json")) else { return false };
        let (entries, active) = if let Ok(f) = serde_json::from_slice::<SessionFile>(&d) {
            (f.docs, f.active)
        } else if let Ok(f) = serde_json::from_slice::<SessionFileV1>(&d) {
            (
                vec![SessionDoc {
                    file_name: f.file_name,
                    original_file: f.original_file,
                    state: f.state,
                    project_path: f.project_path,
                    dirty: f.dirty,
                    file_path: None,
                }],
                0,
            )
        } else {
            return false;
        };
        if entries.is_empty() {
            return false;
        }
        self.restoring = true;
        for e in entries {
            let Ok(data) = std::fs::read(dir.join(&e.original_file)) else { continue };
            let mut slot = DocSlot::new(Arc::new(data), &e.file_name, e.file_path.map(PathBuf::from), None);
            slot.state = e.state;
            slot.project_path = e.project_path.map(PathBuf::from);
            slot.is_dirty = e.dirty;
            slot.session_file = Some(e.original_file);
            self.docs.push(slot);
        }
        let target = self.docs.get(active).or(self.docs.first()).map(|d| d.id);
        if let Some(t) = target {
            self.activate(t);
        }
        let ids: Vec<Uuid> = self.docs.iter().map(|d| d.id).collect();
        for id in ids {
            self.make_strip_thumb(id);
        }
        self.restoring = false;
        // a v1 session used a different file name: rewrite in the new format
        self.save_session(false);
        self.has_image()
    }

    /// Opens images and/or projects; each becomes a new image in the filmstrip.
    pub fn open_paths(&mut self, paths: Vec<PathBuf>) {
        let mut failed: Vec<String> = vec![];
        let mut first = true;
        let n = paths.len();
        for p in paths {
            let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
            let r = if ext == "effector" {
                self.open_project(&p, first).map(|_| ())
            } else {
                SourceImage::load_path(&p).map(|img| {
                    self.open_image(img, Some(p.clone()), DocumentState::default(), None, first);
                })
            };
            match r {
                Ok(()) => first = false,
                Err(_) => failed.push(p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default()),
            }
        }
        if !failed.is_empty() {
            self.alert(&tr("Could not open file"), &failed.join("\n"));
        }
        if n > 1 && !first {
            self.flash(&trf("%d images opened", &[&(n - failed.len())]));
        }
    }

    pub fn open_panel(&mut self) {
        let mut exts: Vec<&str> = imageio::IMAGE_EXTS.to_vec();
        exts.extend(imageio::RAW_EXTS);
        exts.push("effector");
        if let Some(files) = rfd::FileDialog::new().add_filter(&tr("Images and projects"), &exts).pick_files() {
            self.open_paths(files);
        }
    }

    pub fn paste_image(&mut self) {
        let Ok(mut cb) = arboard::Clipboard::new() else { return };
        // files copied in the file manager
        if let Ok(files) = cb.get().file_list() {
            let imgs: Vec<PathBuf> = files
                .into_iter()
                .filter(|p| p.extension().map(|e| imageio::is_image_ext(&e.to_string_lossy()) || e == "effector").unwrap_or(false))
                .collect();
            if !imgs.is_empty() {
                self.open_paths(imgs);
                return;
            }
        }
        if let Ok(img) = cb.get_image() {
            let s = imageio::from_rgba8(img.bytes.into_owned(), img.width, img.height, "Pasted Image.png");
            self.open_image(s, None, DocumentState::default(), None, true);
            return;
        }
        self.flash(&tr("No image on the clipboard"));
    }

    // MARK: - Editing & undo

    /// Applies an edit. Edits with the same `coalesce` key in quick succession share one undo step.
    pub fn edit(&mut self, coalesce: Option<String>, body: impl FnOnce(&mut DocumentState)) {
        let browsing = self.browsing;
        let push_merge = self.browse_undo_pushed;
        let now = Instant::now();
        let merge_key = coalesce.is_some()
            && coalesce == self.last_coalesce_key
            && now.duration_since(self.last_coalesce_time) < Duration::from_secs(1);
        let Some(i) = self.active_index() else { return };
        let d = &mut self.docs[i];
        let mut s = d.state.clone();
        body(&mut s);
        // layers that arrive carrying a mask (presets, pasted layers, old depth masks) get a document mask
        s.adopt_masks();
        if s == d.state {
            return;
        }
        let mut merge = merge_key;
        if browsing {
            // all changes made while browsing a picker form one undo step
            merge = push_merge;
            self.browse_undo_pushed = true;
            self.interactive_until = now + Duration::from_millis(200);
        }
        if !merge {
            d.undo.push(d.state.clone());
            if d.undo.len() > 300 {
                d.undo.remove(0);
            }
            d.redo.clear();
        }
        self.last_coalesce_key = coalesce.clone();
        self.last_coalesce_time = now;
        if coalesce.is_some() {
            self.interactive_until = now + Duration::from_millis(250);
        }
        let old = std::mem::replace(&mut d.state, s);
        self.state_changed(&old);
    }

    /// Picker browsing session: every change until `end_browse` is a single undo step.
    pub fn begin_browse(&mut self) {
        self.browsing = true;
        self.browse_undo_pushed = false;
    }
    pub fn end_browse(&mut self) {
        self.browsing = false;
        self.browse_undo_pushed = false;
        self.last_coalesce_key = None;
    }

    pub fn undo(&mut self) {
        let Some(i) = self.active_index() else { return };
        let d = &mut self.docs[i];
        let Some(prev) = d.undo.pop() else { return };
        let cur = std::mem::replace(&mut d.state, prev);
        d.redo.push(cur.clone());
        self.last_coalesce_key = None;
        self.state_changed(&cur);
        if let Some(id) = self.selected_id {
            if self.layers().find(id).is_none() {
                self.set_selected(None);
            }
        }
    }

    pub fn redo(&mut self) {
        let Some(i) = self.active_index() else { return };
        let d = &mut self.docs[i];
        let Some(next) = d.redo.pop() else { return };
        let cur = std::mem::replace(&mut d.state, next);
        d.undo.push(cur.clone());
        self.last_coalesce_key = None;
        self.state_changed(&cur);
    }

    fn state_changed(&mut self, old: &DocumentState) {
        // A real edit always shows the real stack: a hover preview left behind (e.g. a preset
        // highlighted while its search field kept focus) would be drawn on top and hide the edit.
        self.hover_pending = None;
        self.preview_layers = None;
        // browsers forget their highlighted card, so hovering it again previews it again
        self.hover_epoch += 1;
        let new = self.state();
        if let Some(d) = self.doc_mut() {
            d.is_dirty = true;
        }
        self.thumb_due = Some(Instant::now() + Duration::from_millis(700));
        self.schedule_session_save();
        if old.crop != new.crop || old.canvas != new.canvas {
            self.update_preview_scale(true);
        }
        self.request_render();
    }

    /// Changes that should not create undo steps (UI state like expansion).
    pub fn update_layer_silently(&mut self, id: Uuid, body: impl FnMut(&mut LayerNode)) {
        let mut body = body;
        if let Some(d) = self.doc_mut() {
            d.state.layers.update(id, &mut body);
        }
        self.schedule_session_save();
        self.request_render();
    }

    // MARK: - Layer operations

    pub fn selected_layer(&self) -> Option<LayerNode> { self.selected_id.and_then(|id| self.layers().find(id).cloned()) }

    /// Setting the primary selection collapses a multi-selection.
    pub fn set_selected(&mut self, id: Option<Uuid>) {
        self.selected_id = id;
        self.selected_ids = id.into_iter().collect();
    }

    pub fn add_effect(&mut self, id: &str) {
        if !self.has_image() {
            return;
        }
        let node = LayerNode::effect(id, None, None);
        self.insert_above_selection(node);
    }

    pub fn add_lut_layer(&mut self, l: LutData) {
        let mut p = Params::default();
        p.set("lut", ParamValue::Lut(l.clone()));
        let node = LayerNode::effect("lut", Some(l.name.clone()), Some(p));
        self.insert_above_selection(node);
        self.flash(&trf("LUT added: %@", &[&l.name]));
    }

    pub fn insert_above_selection(&mut self, node: LayerNode) {
        let sel = self.selected_id;
        let nid = node.id;
        self.edit(None, |s| {
            if let Some(sel) = sel {
                if let Some((parent, index)) = s.layers.location(sel) {
                    let is_open_group = s.layers.find(sel).map(|n| n.is_group && n.expanded).unwrap_or(false);
                    if is_open_group {
                        s.layers.insert_node(node, Some(sel), 0);
                    } else {
                        s.layers.insert_node(node, parent, index);
                    }
                    return;
                }
            }
            s.layers.insert(0, node);
        });
        self.set_selected(Some(nid));
    }

    pub fn update_layer(&mut self, id: Uuid, coalesce: Option<String>, body: impl FnMut(&mut LayerNode)) {
        let mut body = body;
        self.edit(coalesce, |s| {
            s.layers.update(id, &mut body);
        });
    }

    pub fn set_param(&mut self, id: Uuid, key: &str, value: ParamValue, coalesce: bool) {
        let c = if coalesce { Some(format!("{id}-{key}")) } else { None };
        let key = key.to_string();
        self.update_layer(id, c, |n| n.params.set(&key, value.clone()));
    }

    // MARK: Selection

    pub fn is_multi_selection(&self) -> bool { self.selected_ids.len() > 1 }

    /// Click on a layer row: plain = select, shift = range, cmd = toggle.
    pub fn click(&mut self, id: Uuid, m: egui::Modifiers) {
        if m.shift {
            if let Some(anchor) = self.selection_anchor.or(self.selected_id) {
                let order: Vec<Uuid> = self.layers().flattened().iter().map(|r| r.0.id).collect();
                if let (Some(a), Some(b)) = (order.iter().position(|x| *x == anchor), order.iter().position(|x| *x == id)) {
                    self.selected_ids = order[a.min(b)..=a.max(b)].iter().copied().collect();
                    self.selected_id = Some(id);
                    return;
                }
            }
        }
        if m.command {
            if self.selected_ids.contains(&id) {
                self.selected_ids.remove(&id);
                if self.selected_id == Some(id) {
                    self.selected_id = self.order(&self.selected_ids).first().copied();
                }
            } else {
                self.selected_ids.insert(id);
                self.selected_id = Some(id);
            }
            self.selection_anchor = Some(id);
            return;
        }
        self.set_selected(Some(id));
        self.selection_anchor = Some(id);
    }

    pub fn select_all_layers(&mut self) {
        let all: Vec<Uuid> = self.layers().flattened().iter().map(|r| r.0.id).collect();
        self.selected_ids = all.iter().copied().collect();
        self.selected_id = all.first().copied();
    }

    fn all_ids(nodes: &[LayerNode], out: &mut Vec<Uuid>) {
        for n in nodes {
            out.push(n.id);
            Self::all_ids(&n.children, out);
        }
    }

    fn order(&self, ids: &HashSet<Uuid>) -> Vec<Uuid> {
        let mut all = vec![];
        Self::all_ids(self.layers(), &mut all);
        all.into_iter().filter(|i| ids.contains(i)).collect()
    }

    /// Selected nodes that are not inside another selected node, top-first.
    pub fn selection_roots(&self) -> Vec<Uuid> {
        let sel: HashSet<Uuid> = if self.selected_ids.is_empty() { self.selected_id.into_iter().collect() } else { self.selected_ids.clone() };
        let layers = self.layers();
        self.order(&sel).into_iter().filter(|id| !sel.iter().any(|o| o != id && layers.contains_inside(*id, *o))).collect()
    }

    pub fn delete_selected(&mut self) {
        let roots = self.selection_roots();
        let Some(first) = roots.first().copied() else { return };
        let loc = self.layers().location(first);
        let r2 = roots.clone();
        self.edit(None, |s| {
            for id in &r2 {
                s.layers.remove_node(*id);
            }
        });
        match loc {
            Some((parent, index)) => {
                let siblings: Vec<LayerNode> =
                    parent.and_then(|p| self.layers().find(p).map(|n| n.children.clone())).unwrap_or_else(|| self.layers().to_vec());
                if siblings.is_empty() {
                    self.set_selected(parent);
                } else {
                    self.set_selected(Some(siblings[index.min(siblings.len() - 1)].id));
                }
            }
            None => self.set_selected(None),
        }
    }

    pub fn duplicate_selected(&mut self) {
        let roots = self.selection_roots();
        if roots.is_empty() {
            return;
        }
        let mut copies = vec![];
        let copy_word = tr("copy");
        self.edit(None, |s| {
            for id in &roots {
                let (Some(n), Some((parent, index))) = (s.layers.find(*id).cloned(), s.layers.location(*id)) else { continue };
                let mut c = n.reidentified();
                c.name = format!("{} {}", n.name, copy_word);
                copies.push(c.id);
                s.layers.insert_node(c, parent, index);
            }
        });
        self.selected_ids = copies.iter().copied().collect();
        self.selected_id = copies.first().copied();
    }

    /// Groups the selection (all selected layers go into one new group, keeping their order).
    pub fn group_selected(&mut self) {
        if !self.has_image() {
            return;
        }
        let roots = self.selection_roots();
        let loc = roots.first().and_then(|f| self.layers().location(*f));
        let Some((parent, index)) = loc else {
            let g = LayerNode::group(&tr("Group"), vec![]);
            let gid = g.id;
            self.edit(None, |s| s.layers.insert(0, g));
            self.set_selected(Some(gid));
            return;
        };
        let nodes: Vec<LayerNode> = roots.iter().filter_map(|r| self.layers().find(*r).cloned()).collect();
        let g = LayerNode::group(&tr("Group"), nodes);
        let gid = g.id;
        self.edit(None, |s| {
            for id in &roots {
                s.layers.remove_node(*id);
            }
            let parent_ok = parent.map(|p| s.layers.find(p).is_some()).unwrap_or(true);
            if parent_ok {
                let count = parent.and_then(|p| s.layers.find(p).map(|n| n.children.len())).unwrap_or(s.layers.len());
                s.layers.insert_node(g, parent, index.min(count));
            } else {
                s.layers.insert(0, g);
            }
        });
        self.set_selected(Some(gid));
    }

    /// Applies a change to every selected layer as one undo step.
    pub fn update_selected(&mut self, coalesce: Option<String>, body: impl FnMut(&mut LayerNode)) {
        let ids = self.selection_roots();
        let mut body = body;
        self.edit(coalesce, |s| {
            for id in &ids {
                s.layers.update(*id, &mut body);
            }
        });
    }

    pub fn toggle_selected_visibility(&mut self) {
        let ids = self.selection_roots();
        let Some(first) = ids.first().and_then(|f| self.layers().find(*f)) else { return };
        let on = !first.enabled;
        self.edit(None, |s| {
            for id in &ids {
                s.layers.update(*id, &mut |n| n.enabled = on);
            }
        });
    }

    pub fn ungroup_selected(&mut self) {
        let Some(id) = self.selected_id else { return };
        let Some(g) = self.layers().find(id).cloned() else { return };
        if !g.is_group {
            return;
        }
        let Some((parent, index)) = self.layers().location(id) else { return };
        let g2 = g.clone();
        self.edit(None, |s| {
            s.layers.remove_node(id);
            for (i, c) in g2.children.iter().enumerate() {
                let mut c = c.clone();
                if g2.opacity < 1.0 {
                    c.opacity *= g2.opacity;
                }
                s.layers.insert_node(c, parent, index + i);
            }
        });
        self.set_selected(g.children.first().map(|c| c.id));
    }

    pub fn toggle_visibility(&mut self, id: Uuid) { self.update_layer(id, None, |n| n.enabled = !n.enabled); }

    pub fn reset_params(&mut self, id: Uuid) {
        let Some(n) = self.layers().find(id).cloned() else { return };
        if n.is_group {
            return;
        }
        let fresh = LayerNode::effect(&n.effect_id, None, None);
        self.update_layer(id, None, |l| l.params = fresh.params.clone());
    }

    /// Resets the layer to the effect defaults and applies a look's values.
    pub fn apply_look(&mut self, id: Uuid, look: &Look) {
        let Some(n) = self.layers().find(id).cloned() else { return };
        if n.is_group {
            return;
        }
        let mut p = LayerNode::effect(&n.effect_id, None, None).params;
        for (k, v) in &look.values {
            p.set(k, v.clone());
        }
        // keep the seed so browsing looks doesn't reshuffle noise
        if let Some(seed) = n.params.get("seed") {
            p.set("seed", seed.clone());
        }
        p.set("_look", ParamValue::Text(look.id.clone()));
        self.update_layer(id, None, |l| l.params = p.clone());
    }

    /// Moves several layers (kept in order) relative to `target`.
    pub fn move_layers(&mut self, ids: &[Uuid], target: Uuid, pos: DropPosition) {
        let layers = self.layers().to_vec();
        let ids: Vec<Uuid> = ids.iter().copied().filter(|id| *id != target && !layers.contains_inside(target, *id)).collect();
        if ids.is_empty() {
            return;
        }
        self.edit(None, |s| {
            let nodes: Vec<LayerNode> = ids.iter().filter_map(|id| s.layers.remove_node(*id)).collect();
            let Some((parent, index)) = s.layers.location(target) else {
                for n in nodes.into_iter().rev() {
                    s.layers.insert(0, n);
                }
                return;
            };
            for (i, n) in nodes.into_iter().enumerate() {
                match pos {
                    DropPosition::Above => s.layers.insert_node(n, parent, index + i),
                    DropPosition::Below => s.layers.insert_node(n, parent, index + 1 + i),
                    DropPosition::Inside => s.layers.insert_node(n, Some(target), i),
                }
            }
        });
        if pos == DropPosition::Inside {
            self.update_layer_silently(target, |n| n.expanded = true);
        }
    }

    pub fn move_to_root_bottom(&mut self, ids: &[Uuid]) {
        let ids = ids.to_vec();
        self.edit(None, |s| {
            let nodes: Vec<LayerNode> = ids.iter().filter_map(|id| s.layers.remove_node(*id)).collect();
            s.layers.extend(nodes);
        });
    }

    /// Move selected layer up / down among siblings (keyboard).
    pub fn nudge_selected(&mut self, delta: i64) {
        let Some(id) = self.selected_id else { return };
        let Some((parent, index)) = self.layers().location(id) else { return };
        self.edit(None, |s| {
            let Some(n) = s.layers.remove_node(id) else { return };
            s.layers.insert_node(n, parent, (index as i64 + delta).max(0) as usize);
        });
    }

    /// Saves the whole layer stack as a user preset (asks for a name).
    pub fn save_stack_as_preset(&mut self) {
        if !self.has_image() || self.layers().is_empty() {
            return;
        }
        self.dialog = Some(Dialog::Prompt { title: tr("Save as Preset"), label: tr("Name"), text: tr("My Preset"), kind: PromptKind::SaveStackPreset });
    }

    /// Adds layers as a user preset with a thumbnail rendered from the current image.
    pub fn add_preset(&mut self, name: &str, layers: &[LayerNode]) {
        // masks travel inside the preset, so it works on other images too
        let layers = &self.state().portable(layers);
        let thumb = self.render_thumbnail_blocking(&resolve_masks(layers, &[]), 240.0).and_then(|(px, w, h)| {
            let rgb: Vec<u8> = px.chunks(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
            let mut out = vec![];
            jpeg_encoder::Encoder::new(&mut out, 80).encode(&rgb, w as u16, h as u16, jpeg_encoder::ColorType::Rgb).ok()?;
            Some(out)
        });
        presets::add(name, layers, thumb);
        self.regenerate_preset_thumbs();
    }

    /// Temporary preview while hovering / browsing (None clears it, with a short delay so moving
    /// between cards doesn't flicker back to the original).
    pub fn hover_preview(&mut self, layers: Option<Vec<LayerNode>>) {
        if !settings::get().hover_preview && layers.is_some() {
            return;
        }
        let delay = if layers.is_none() { 180 } else { 60 };
        self.hover_pending = Some((layers, Instant::now() + Duration::from_millis(delay)));
        // the pointer may stop right on the card: without this no frame comes to apply it
        self.ctx.request_repaint_after(Duration::from_millis(delay + 5));
    }

    pub fn set_preview_layers(&mut self, layers: Option<Vec<LayerNode>>) {
        self.hover_pending = None;
        if self.preview_layers != layers {
            self.preview_layers = layers;
            self.request_render();
        }
    }

    pub fn preset_preview_layers(&self, p: &Preset) -> Vec<LayerNode> {
        let mut g = LayerNode::group(&p.display_name(), p.layers.clone());
        g.expanded = false;
        vec![g]
    }

    pub fn apply_preset(&mut self, p: &Preset) {
        if !self.has_image() {
            return;
        }
        let mut g = LayerNode::group(&p.display_name(), p.layers.iter().map(|l| l.reidentified()).collect());
        g.expanded = false;
        let gid = g.id;
        self.edit(None, |s| s.layers.insert(0, g));
        self.set_selected(Some(gid));
        self.flash(&trf("Preset applied: %@", &[&p.display_name()]));
    }

    // MARK: - Rendering

    pub fn request_render(&mut self) { self.render_pending = true; }

    /// Dev/test: one frame of a simulated slider drag on the selected layer.
    pub fn debug_drag_step(&mut self, n: u64) {
        let Some(id) = self.selected_id else { return };
        let Some(node) = self.layers().find(id).cloned() else { return };
        let Some(eff) = effects::effect(&node.effect_id) else { return };
        let sliders: Vec<_> = eff.params.iter().filter_map(|d| match &d.kind {
            ParamKind::Slider { min, max, .. } => Some((d.key.clone(), *min, *max)),
            _ => None,
        }).collect();
        if sliders.is_empty() { return; }
        let (k, min, max) = &sliders[(n / 40) as usize % sliders.len()];
        let t = ((n % 40) as f64) / 39.0;
        self.set_param(id, k, ParamValue::Number(min + (max - min) * t), true);
    }

    /// Dev/test: browse to gradient number `n` on the selected layer (like the gradient picker).
    pub fn debug_gradient_step(&mut self, n: usize) {
        let Some(id) = self.selected_id else { return };
        let all = crate::swatches::all_gradients();
        let g = all[n % all.len()].clone();
        self.set_param(id, "gradient", ParamValue::Gradient(g), false);
    }

    pub fn debug_render_state(&self) -> String {
        format!(
            "render: gen={} pending={} running={} is_rendering={} display_gen={} has_output={}",
            self.render_gen, self.render_pending, self.render_running, self.is_rendering, self.display_gen, self.output.is_some()
        )
    }

    /// Does the stack (or the effect / preset being previewed) draw a fine pixel pattern?
    fn has_fine_pattern(&self) -> bool {
        fn any(nodes: &[LayerNode]) -> bool {
            nodes.iter().any(|n| n.enabled && (any(&n.children) || (!n.is_group && crate::effects::effect(&n.effect_id).map(|e| e.fine_pattern).unwrap_or(false))))
        }
        any(self.layers()) || self.preview_layers.as_deref().map(any).unwrap_or(false)
    }

    /// Size of the image on screen in physical pixels (None before the first layout).
    fn display_target(&self) -> Option<(usize, usize)> {
        let (w, h) = self.output_size();
        let z = self.display_zoom() * self.pixels_per_point;
        if w == 0 || z <= 0.0 || self.viewport.x <= 10.0 {
            return None;
        }
        Some((((w as f64 * z).round() as usize).max(1), ((h as f64 * z).round() as usize).max(1)))
    }

    fn current_scale(&mut self) -> f64 {
        let mut s = self.preview_scale;
        // dots, dither and characters at a reduced scale make patterns the export doesn't have:
        // render them at full resolution and let the display copy shrink them properly
        let fine = self.has_fine_pattern();
        if fine {
            s = 1.0;
        }
        let big = self.source().map(|x| x.width.max(x.height) > 4500).unwrap_or(false);
        if Instant::now() < self.interactive_until && s > 0.3 && (!fine || big) {
            s *= 0.5;
            self.refine_at = Some(Instant::now() + Duration::from_millis(300));
        }
        s
    }

    fn kick_render(&mut self) {
        if !self.render_pending || self.render_running {
            return;
        }
        let Some(src) = self.source() else { return };
        self.render_pending = false;
        self.render_running = true;
        self.render_started = Instant::now();
        self.is_rendering = true;
        let st = self.state();
        let mut layers = st.render_layers();
        if let Some(pv) = &self.preview_layers {
            let mut l = resolve_masks(pv, &st.masks);
            l.extend(layers);
            layers = l;
        }
        let scale = self.current_scale();
        self.render_gen += 1;
        self.render.request(RenderRequest {
            generation: self.render_gen,
            layers,
            source: src,
            crop: st.crop,
            canvas: st.active_canvas().cloned(),
            scale,
            want_crop_view: self.is_cropping,
            show_clipping: self.show_clipping,
            want_histogram: settings::get().histogram_open,
            mask_overlay: self.mask_overlay(),
            display_size: self.display_target(),
        });
        self.last_display_target = self.display_target();
    }

    /// Starts subject detection on the active image when something needs it; re-renders when it lands.
    fn tick_subject(&mut self, ctx: &egui::Context) {
        let Some(src) = self.source() else { return };
        let st = self.state();
        let mut needs = self.mask_overlay().map(|m| m.needs_subject()).unwrap_or(false) || crate::subject::layers_need_subject(&st.render_layers());
        if let Some(pv) = &self.preview_layers {
            needs |= crate::subject::layers_need_subject(&resolve_masks(pv, &st.masks));
        }
        if needs && crate::subject::get(src.id).is_none() {
            if !crate::subject::model_available() {
                if !self.subject_missing_warned {
                    self.subject_missing_warned = true;
                    self.flash(&tr("Subject detection model not found: reinstall EFFECTOR"));
                }
            } else {
                let c = ctx.clone();
                crate::subject::request(&src, move || c.request_repaint());
            }
        }
        let g = crate::subject::generation();
        if g != self.subject_gen {
            self.subject_gen = g;
            self.request_render();
            if let Some(id) = self.active_id {
                self.make_strip_thumb(id);
            }
        }
        if crate::subject::is_running(src.id) {
            ctx.request_repaint_after(Duration::from_millis(200));
        }
    }

    /// Subject detection is running on the active image.
    pub fn subject_busy(&self) -> bool { self.source().map(|s| crate::subject::is_running(s.id)).unwrap_or(false) }

    /// Picks a preview resolution matching what is on screen (quantized to avoid churn).
    pub fn output_size(&self) -> (usize, usize) {
        let Some(src) = self.source() else { return (0, 0) };
        let st = self.state();
        let (w, h) = src.cropped_size(&st.crop);
        match st.active_canvas() {
            None => (w, h),
            Some(c) => {
                let (s, _) = c.layout(w as f64, h as f64);
                (s.0 as usize, s.1 as usize)
            }
        }
    }

    pub fn update_preview_scale(&mut self, force: bool) {
        if self.source().is_none() || self.viewport.x <= 10.0 {
            return;
        }
        let (cw, ch) = self.output_size();
        let long_side = cw.max(ch) as f64;
        let fit = (self.viewport.x as f64 / cw as f64).min(self.viewport.y as f64 / ch as f64);
        self.fit_zoom = fit;
        let z = self.zoom().unwrap_or(fit);
        let mut needed = z * self.pixels_per_point;
        needed = needed.max(600.0 / long_side).min(1.0);
        // quantize to 1/4 octave steps
        let q = 2f64.powf((needed.log2() * 4.0).ceil() / 4.0);
        let new_scale = q.min(1.0);
        if force || (new_scale - self.preview_scale).abs() > 1e-6 {
            self.preview_scale = new_scale;
            self.request_render();
        }
    }

    pub fn display_zoom(&self) -> f64 { self.zoom().unwrap_or(self.fit_zoom) }

    pub fn set_zoom(&mut self, z: Option<f64>) {
        if let Some(d) = self.doc_mut() {
            match z {
                Some(z) => d.zoom = Some(z.clamp(0.02, 32.0)),
                None => {
                    d.zoom = None;
                    d.pan = egui::Vec2::ZERO;
                }
            }
        }
        self.update_preview_scale(false);
    }

    pub fn zoom_in(&mut self) { self.set_zoom(Some(self.display_zoom() * 1.25)); }
    pub fn zoom_out(&mut self) { self.set_zoom(Some(self.display_zoom() / 1.25)); }
    pub fn zoom_actual(&mut self) { self.set_zoom(Some(1.0 / self.pixels_per_point)); }

    /// Called every frame: timers, worker results, pending renders.
    pub fn tick(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        // Image Overlay: the built-in textures are downloaded the first time a layer uses them
        if !crate::textures::is_ready() && crate::textures::download_state() == crate::textures::Download::Idle {
            let uses = self.has_image()
                && (crate::textures::layers_use_library(self.layers()) || self.preview_layers.as_deref().map(crate::textures::layers_use_library).unwrap_or(false));
            if uses {
                crate::textures::ensure_library(false);
            }
        }
        if crate::textures::generation() != self.tex_gen {
            self.tex_gen = crate::textures::generation();
            self.request_render();
            self.regenerate_thumbnails();
        }
        for name in crate::model::take_removed_notice() {
            self.flash(&trf("%@ was removed from EFFECTOR and was left out", &[&tr(name)]));
        }
        // render results
        // Watchdog: a render thread that died or hangs must never freeze the preview for good.
        if self.render_running && (self.render.is_dead() || now.duration_since(self.render_started) > Duration::from_secs(20)) {
            settings::log_error(if self.render.is_dead() { "render thread died, restarting it" } else { "render took over 20 s, restarting the render thread" });
            self.render = RenderWorker::start(ctx.clone());
            self.render_running = false;
            self.render_pending = true;
        }
        while let Ok(msg) = self.render.results.try_recv() {
            self.render_running = false;
            let r = match msg {
                RenderMsg::Done(r) => r,
                RenderMsg::Failed(generation) => {
                    settings::log_error(&format!("render {generation} failed"));
                    self.is_rendering = self.render_pending;
                    continue;
                }
            };
            if r.generation == self.render_gen || self.output.is_none() {
                self.output = Some(r.output);
                self.original = Some(r.original);
                self.display = Some(r.display);
                self.display_original = Some(r.display_original);
                if r.histogram.is_some() {
                    self.histogram = r.histogram;
                }
                if r.crop_view.is_some() {
                    self.crop_view = r.crop_view;
                }
                self.mask_view = r.mask_view;
                self.display_gen += 1;
            }
            self.is_rendering = self.render_pending;
        }
        // thumbnails
        while let Ok(t) = self.thumbs.results.try_recv() {
            match t {
                ThumbResult::Effect { generation, id, rgba, w, h } => {
                    if generation == self.effects_gen {
                        let tex = ctx.load_texture(format!("fx-{id}"), egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba), Default::default());
                        self.effect_thumbs.insert(id, tex);
                    }
                }
                ThumbResult::Preset { generation, id, rgba, w, h } => {
                    if generation == self.presets_gen {
                        let tex = ctx.load_texture(format!("pr-{id}"), egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba), Default::default());
                        self.preset_thumbs.insert(id, tex);
                    }
                }
                ThumbResult::Strip { doc, rgba, w, h } => {
                    let tex = ctx.load_texture(format!("strip-{doc}"), egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba), Default::default());
                    if let Some(d) = self.docs.iter_mut().find(|d| d.id == doc) {
                        d.thumb = Some(tex);
                    }
                }
            }
        }
        if let Some((layers, at)) = self.hover_pending.clone() {
            if now >= at {
                self.set_preview_layers(layers);
            } else {
                ctx.request_repaint_after(at - now);
            }
        }
        if let Some(at) = self.refine_at {
            if now >= at {
                self.refine_at = None;
                self.request_render();
            } else {
                ctx.request_repaint_after(at - now);
            }
        }
        if let Some(at) = self.thumb_due {
            if now >= at {
                self.thumb_due = None;
                if let Some(id) = self.active_id {
                    self.make_strip_thumb(id);
                }
            } else {
                ctx.request_repaint_after(at - now);
            }
        }
        if let Some(at) = self.session_due {
            if now >= at {
                self.session_due = None;
                self.save_session(false);
            } else {
                ctx.request_repaint_after(at - now);
            }
        }
        if let Some((_, at)) = &self.toast {
            if now.duration_since(*at) > Duration::from_millis(2200) {
                self.toast = None;
            } else {
                ctx.request_repaint_after(Duration::from_millis(200));
            }
        }
        if let Some((c, total)) = &self.export_counter {
            self.export_progress = Some(format!("{} / {total}", c.load(std::sync::atomic::Ordering::Relaxed).min(*total)));
        }
        if let Some(rx) = &self.export_rx {
            if let Ok(r) = rx.try_recv() {
                self.export_rx = None;
                self.is_exporting = false;
                self.export_progress = None;
                self.export_counter = None;
                self.show_export = false;
                match r {
                    Ok(msg) => self.flash(&msg),
                    Err(e) => self.alert(&tr("Export failed"), &e),
                }
            } else {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }
        if let Some(rx) = &self.copy_rx {
            if let Ok(r) = rx.try_recv() {
                self.copy_rx = None;
                self.is_exporting = false;
                if let Some((px, w, h)) = r {
                    if let Ok(mut cb) = arboard::Clipboard::new() {
                        let _ = cb.set_image(arboard::ImageData { width: w, height: h, bytes: std::borrow::Cow::Owned(px) });
                        self.flash(&tr("Image copied to clipboard"));
                    }
                }
            } else {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }
        self.tick_subject(ctx);
        if self.update_check_auto && !self.update_announced {
            match crate::updater::current() {
                crate::updater::State::Available { version, .. } => {
                    self.update_announced = true;
                    if settings::get().skipped_version != version {
                        self.show_update_popup = true;
                    }
                }
                crate::updater::State::UpToDate | crate::updater::State::Failed(_) => self.update_announced = true,
                _ => {}
            }
        }
        // the display copy is made for the on-screen size: zooming asks for a new one (the layers
        // come from the render cache, so only the shrinking runs again)
        if self.has_image() && !self.render_pending && self.display_target() != self.last_display_target {
            self.request_render();
        }
        self.kick_render();
        if self.render_running || self.render_pending {
            ctx.request_repaint_after(Duration::from_millis(30));
        }
    }

    // MARK: Randomize / copy & paste layers

    /// Rolls new random values for a layer's sliders, menus and seed (one undo step).
    pub fn randomize_params(&mut self, id: Uuid) {
        let Some(n) = self.layers().find(id).cloned() else { return };
        if n.is_group {
            return;
        }
        let Some(eff) = effects::effect(&n.effect_id) else { return };
        let mut rng = crate::swatches::SplitMix::new(rand_seed());
        let mut p = n.params.clone();
        for d in &eff.params {
            match &d.kind {
                ParamKind::Slider { min, max, step, curve, .. } => {
                    let mut v = min + (max - min) * rng.unit().powf(*curve);
                    if *step > 0.0 {
                        v = (v / step).round() * step;
                    }
                    p.set(&d.key, ParamValue::Number(v.clamp(*min, *max)));
                }
                ParamKind::Choice(o, _) | ParamKind::Segmented(o, _) => {
                    if !o.is_empty() {
                        let r = (rng.next() % o.len() as u64) as usize;
                        p.set(&d.key, ParamValue::Choice(o[r].id.clone()));
                    }
                }
                ParamKind::Seed => p.set(&d.key, ParamValue::Number((1 + rng.next() % 9999) as f64)),
                _ => {}
            }
        }
        p.values.remove("_look");
        self.update_layer(id, None, |l| l.params = p.clone());
    }

    pub fn copy_layers(&mut self) {
        let nodes: Vec<LayerNode> = self.selection_roots().iter().filter_map(|id| self.layers().find(*id).cloned()).collect();
        if nodes.is_empty() {
            return;
        }
        let nodes = self.state().portable(&nodes);
        if let (Ok(json), Ok(mut cb)) = (serde_json::to_string(&nodes), arboard::Clipboard::new()) {
            let _ = cb.set_text(format!("{LAYERS_CLIP_PREFIX}{json}"));
            self.flash(&trf("%d layers copied", &[&nodes.len()]));
        }
    }

    pub fn paste_layers(&mut self) {
        if !self.has_image() {
            return;
        }
        let nodes: Option<Vec<LayerNode>> = arboard::Clipboard::new()
            .ok()
            .and_then(|mut c| c.get_text().ok())
            .and_then(|t| t.strip_prefix(LAYERS_CLIP_PREFIX).map(|s| s.to_string()))
            .and_then(|j| serde_json::from_str(&j).ok());
        let Some(nodes) = nodes else {
            self.flash(&tr("No layers to paste"));
            return;
        };
        let fresh: Vec<LayerNode> = nodes.iter().map(|n| n.reidentified()).collect();
        let sel = self.selected_id;
        let f2 = fresh.clone();
        self.edit(None, |s| {
            let (mut parent, mut index) = (None, 0);
            if let Some((p, i)) = sel.and_then(|x| s.layers.location(x)) {
                parent = p;
                index = i;
            }
            for (i, n) in f2.into_iter().enumerate() {
                s.layers.insert_node(n, parent, index + i);
            }
        });
        self.selected_ids = fresh.iter().map(|n| n.id).collect();
        self.selected_id = fresh.first().map(|n| n.id);
        self.flash(&trf("%d layers pasted", &[&fresh.len()]));
    }

    pub fn toggle_presentation(&mut self, ctx: &egui::Context) {
        if !self.has_image() && !self.presenting {
            return;
        }
        self.show_effect_browser = false;
        self.show_command_palette = false;
        self.presenting = !self.presenting;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.presenting));
        if self.presenting {
            self.set_zoom(None);
            self.flash(&format!("{} · F {} {}", tr("Full screen preview"), tr("or"), if crate::shortcuts::IS_MAC { "⎋" } else { "Esc" }));
        } else {
            self.set_zoom(None);
        }
    }

    // MARK: - Thumbnails

    pub fn regenerate_thumbnails(&mut self) {
        let Some(src) = self.source() else { return };
        self.effects_gen += 1;
        self.effect_thumbs.clear();
        self.thumbs.send(ThumbJob::Effects { source: src, crop: self.state().crop, generation: self.effects_gen });
        self.regenerate_preset_thumbs();
    }

    pub fn regenerate_preset_thumbs(&mut self) {
        let Some(src) = self.source() else { return };
        self.presets_gen += 1;
        let list: Vec<(Uuid, Vec<LayerNode>)> = presets::all().into_iter().map(|p| (p.id, p.layers)).collect();
        self.thumbs.send(ThumbJob::Presets { source: src, crop: self.state().crop, presets: list, generation: self.presets_gen });
    }

    pub fn render_thumbnail_blocking(&self, layers: &[LayerNode], size: f64) -> Option<(Vec<u8>, usize, usize)> {
        let src = self.source()?;
        let (tx, rx) = std::sync::mpsc::channel();
        self.thumbs.send(ThumbJob::Render { source: src, crop: self.state().crop, layers: layers.to_vec(), size, reply: tx });
        rx.recv_timeout(Duration::from_secs(20)).ok().flatten()
    }

    // MARK: - Export

    /// The images to export (in filmstrip order), with what the export needs from each.
    pub fn export_jobs(&self, ids: &HashSet<Uuid>) -> Vec<crate::export::Job> {
        self.docs
            .iter()
            .filter(|d| ids.contains(&d.id))
            .map(|d| crate::export::Job {
                name: d.file_name.clone(),
                data: d.data.clone(),
                source: d.source.clone(),
                state: d.state.clone(),
                file_path: d.file_path.clone(),
                project_path: d.project_path.clone(),
                full_size: d.source.as_ref().map(|s| crate::export::full_size(s, &d.state)).unwrap_or((0, 0)),
                taken: crate::exif::date_taken(&d.data),
            })
            .collect()
    }

    /// Exports in the background; the dialog closes and a message shows when it is done.
    pub fn start_export(&mut self, s: crate::export::ExportSettings, jobs: Vec<crate::export::Job>, planned: Vec<crate::export::Planned>, existing: crate::export::Existing) {
        if planned.is_empty() {
            return;
        }
        self.is_exporting = true;
        let total = planned.len();
        let progress = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        self.export_counter = Some((progress.clone(), total));
        self.export_progress = Some(format!("0 / {total}"));
        let (tx, rx) = std::sync::mpsc::channel();
        self.export_rx = Some(rx);
        let repaint = self.ctx.clone();
        std::thread::spawn(move || {
            let r = crate::export::run(&s, jobs, planned, existing, progress);
            let _ = tx.send(r);
            repaint.request_repaint();
        });
    }

    pub fn copy_result(&mut self) {
        let Some(src) = self.source() else { return };
        self.is_exporting = true;
        let st = self.state();
        let (tx, rx) = std::sync::mpsc::channel();
        self.copy_rx = Some(rx);
        let repaint = self.ctx.clone();
        std::thread::spawn(move || {
            let tex = src.make_texture(&st.crop, 1.0);
            let mut layers = st.render_layers();
            let extras = crate::renderer::Extras::for_render(&src, &st.crop, &layers, true);
            let out = crate::renderer::Renderer::new(false).render(&layers, &tex, crate::worker::content_key(src.id, &st.crop), true, st.active_canvas(), extras);
            let px = imageio::read_rgba8(&out);
            let _ = tx.send(Some((px, out.width(), out.height())));
            repaint.request_repaint();
        });
    }

    // MARK: - Project files

    pub fn save(&mut self, save_as: bool) {
        let Some(d) = self.doc() else { return };
        let mut path = d.project_path.clone();
        if path.is_none() || save_as {
            let stem = Path::new(&d.file_name).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "project".into());
            match rfd::FileDialog::new().set_file_name(format!("{stem}.effector")).add_filter("EFFECTOR", &["effector"]).save_file() {
                Some(p) => path = Some(p),
                None => return,
            }
        }
        let Some(path) = path else { return };
        match self.write_project(&path) {
            Ok(()) => {
                if let Some(d) = self.doc_mut() {
                    d.project_path = Some(path);
                    d.is_dirty = false;
                }
                self.schedule_session_save();
                self.flash(&tr("Saved"));
            }
            Err(e) => self.alert(&tr("Could not save"), &e),
        }
    }

    /// Writes the .effector project: a single file (zip) with document.json, the original image and a thumbnail.
    pub fn write_project(&self, path: &Path) -> Result<(), String> {
        let d = self.doc().ok_or("no image")?;
        let manifest = ProjectManifest { version: 1, app: "EFFECTOR".into(), original_file_name: d.file_name.clone(), state: d.state.clone() };
        let json = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
        let ext = Path::new(&d.file_name).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_else(|| "png".into());
        let thumb = self.render_thumbnail_blocking(&d.state.render_layers(), 512.0).and_then(|(px, w, h)| imageio::encode(&px, w, h, Format::Png, 1.0, true).ok());
        crate::project::write(path, &json, &format!("original.{ext}"), &d.data, thumb.as_deref())
    }

    pub fn open_project(&mut self, path: &Path, activate: bool) -> Result<Uuid, String> {
        let (json, orig_name, data) = crate::project::read(path)?;
        let manifest: ProjectManifest = serde_json::from_slice(&json).map_err(|e| e.to_string())?;
        let _ = orig_name;
        let img = SourceImage::load_data(Arc::new(data), &manifest.original_file_name)?;
        Ok(self.open_image(img, None, manifest.state, Some(path.to_path_buf()), activate))
    }

    // MARK: - Crop

    pub fn begin_crop(&mut self) {
        if !self.has_image() {
            return;
        }
        self.crop_draft = self.state().crop;
        self.is_cropping = true;
        self.tool = Tool::Crop;
        self.show_original = false;
        self.request_render();
    }

    pub fn commit_crop(&mut self) {
        let c = self.crop_draft;
        self.is_cropping = false;
        if self.tool == Tool::Crop {
            self.tool = Tool::Edit;
        }
        self.edit(None, |s| s.crop = c);
        self.set_zoom(None);
        self.regenerate_thumbnails();
    }

    pub fn cancel_crop(&mut self) {
        self.is_cropping = false;
        if self.tool == Tool::Crop {
            self.tool = Tool::Edit;
        }
        self.request_render();
    }

    // MARK: - Tools (right sidebar)

    /// Switches the right sidebar tool. Leaving Crop applies the crop (like Lightroom).
    pub fn set_tool(&mut self, t: Tool) {
        if t == self.tool && !(t == Tool::Crop && !self.is_cropping) {
            return;
        }
        if self.is_cropping && t != Tool::Crop {
            self.commit_crop();
        }
        if t != Tool::Masks {
            self.mask_draw = None;
            self.mask_draw_into = None;
        }
        self.tool = t;
        match t {
            Tool::Crop => {
                if !self.is_cropping {
                    self.begin_crop();
                }
            }
            Tool::Masks => {
                let st = self.state();
                if self.selected_mask.map(|id| st.mask(id).is_none()).unwrap_or(true) {
                    // the mask of the selected layer, else the first one
                    let from_layer = self.selected_layer().and_then(|l| l.mask).map(|r| r.id).filter(|id| st.mask(*id).is_some());
                    self.selected_mask = from_layer.or(st.masks.first().map(|m| m.id));
                }
            }
            _ => {}
        }
        self.request_render();
    }

    /// Same tool again goes back to Edit (shortcuts toggle, like Lightroom).
    pub fn toggle_tool(&mut self, t: Tool) {
        if !self.has_image() {
            return;
        }
        self.set_tool(if self.tool == t { Tool::Edit } else { t });
    }

    // MARK: - Masks

    /// What the Masks tool shows as a red overlay (the selected mask), if anything.
    pub fn mask_overlay(&self) -> Option<Mask> {
        if self.tool != Tool::Masks || !settings::get().mask_overlay {
            return None;
        }
        let id = self.selected_mask?;
        self.state().mask(id).cloned().filter(|m| m.is_active())
    }

    pub fn update_mask(&mut self, id: Uuid, coalesce: Option<String>, body: impl FnOnce(&mut Mask)) {
        self.edit(coalesce, |s| {
            if let Some(m) = s.masks.iter_mut().find(|m| m.id == id) {
                body(m);
            }
        });
    }

    /// Deletes a mask; the layers that used it go back to the whole image.
    pub fn delete_mask(&mut self, id: Uuid) {
        self.edit(None, |s| {
            s.masks.retain(|m| m.id != id);
            fn clear(nodes: &mut [LayerNode], id: Uuid) {
                for n in nodes.iter_mut() {
                    if n.mask.map(|r| r.id == id).unwrap_or(false) {
                        n.mask = None;
                    }
                    clear(&mut n.children, id);
                }
            }
            clear(&mut s.layers, id);
        });
        if self.selected_mask == Some(id) {
            self.selected_mask = self.state().masks.first().map(|m| m.id);
        }
    }

    pub fn duplicate_mask(&mut self, id: Uuid) {
        let Some(mut m) = self.state().mask(id).cloned() else { return };
        let copy_word = tr("copy");
        m.id = Uuid::new_v4();
        m.name = format!("{} {}", m.name, copy_word);
        for p in m.parts.iter_mut() {
            p.id = Uuid::new_v4();
        }
        let nid = m.id;
        self.edit(None, |s| s.masks.push(m));
        self.selected_mask = Some(nid);
    }

    /// Arms a drawing tool of the Masks panel (or disarms with None). Opens the Masks tool.
    pub fn start_mask_draw(&mut self, d: Option<MaskDraw>) {
        if !self.has_image() {
            return;
        }
        if self.tool != Tool::Masks {
            self.set_tool(Tool::Masks);
        }
        self.mask_draw = if self.mask_draw == d && d.is_some() { None } else { d };
        if d != Some(MaskDraw::Brush) {
            return;
        }
        // the brush keeps painting into the selected brush part, if there is one
        let st = self.state();
        let sel_is_brush = self
            .selected_mask
            .and_then(|m| st.mask(m))
            .and_then(|m| m.parts.iter().find(|p| Some(p.id) == self.selected_part))
            .map(|p| matches!(p.shape, MaskShape::Brush { .. }))
            .unwrap_or(false);
        if !sel_is_brush && self.mask_draw_into.is_none() {
            self.selected_part = None;
        }
    }

    /// Adds a part: to `into` (mask, mode) or as a new mask (applied to the layer whose mask slot
    /// asked for it). Selects it. Returns (mask, part).
    pub fn add_mask_part(&mut self, shape: MaskShape, into: Option<(Uuid, MaskMode)>) -> (Uuid, Uuid) {
        if self.tool != Tool::Masks {
            self.set_tool(Tool::Masks);
        }
        let label = tr(shape.kind_label());
        let part = MaskPart::new(shape, into.map(|i| i.1).unwrap_or(MaskMode::Add));
        let pid = part.id;
        let mid = match into.filter(|(m, _)| self.state().mask(*m).is_some()) {
            Some((m, _)) => {
                self.edit(None, |s| {
                    if let Some(mm) = s.masks.iter_mut().find(|x| x.id == m) {
                        mm.parts.push(part);
                    }
                });
                m
            }
            None => {
                let name = self.state().new_mask_name(&label);
                let m = Mask { id: Uuid::new_v4(), name, parts: vec![part] };
                let id = m.id;
                let apply_to = self.mask_new_for.take();
                self.edit(None, |s| {
                    s.masks.push(m);
                    if let Some(l) = apply_to {
                        s.layers.update(l, &mut |n| n.mask = Some(MaskRef { id, invert: false }));
                    }
                });
                id
            }
        };
        self.selected_mask = Some(mid);
        self.selected_part = Some(pid);
        self.request_render();
        (mid, pid)
    }

    /// Changes one part of a mask.
    pub fn update_part(&mut self, mask: Uuid, part: Uuid, coalesce: Option<String>, body: impl FnOnce(&mut MaskPart)) {
        self.edit(coalesce, |s| {
            if let Some(p) = s.masks.iter_mut().find(|m| m.id == mask).and_then(|m| m.parts.iter_mut().find(|p| p.id == part)) {
                body(p);
            }
        });
    }

    /// Removes a part (the whole mask when it was the last one).
    pub fn delete_part(&mut self, mask: Uuid, part: Uuid) {
        let last = self.state().mask(mask).map(|m| m.parts.len() <= 1).unwrap_or(false);
        if last {
            self.delete_mask(mask);
        } else {
            self.update_mask(mask, None, |m| m.parts.retain(|p| p.id != part));
        }
        if self.selected_part == Some(part) {
            self.selected_part = None;
        }
    }

    /// Keys of the Masks tool, before the global shortcuts (Lightroom Classic): [ ] brush size,
    /// ⇧[ ⇧] brush feather, ' inverts the selected part, ⌫ deletes it, Esc stops drawing.
    /// Also: Enter / Esc leave the Masks and Canvas tools back to Edit (Crop handles its own:
    /// Enter applies, Esc cancels, both return to Edit).
    pub fn mask_key(&mut self, combo: &crate::shortcuts::KeyCombo) -> bool {
        if combo.cmd || combo.opt {
            return false;
        }
        let leave = matches!(combo.key.as_str(), "escape" | "return") && !combo.shift;
        if self.tool == Tool::Canvas && leave {
            self.set_tool(Tool::Edit);
            return true;
        }
        if self.tool != Tool::Masks {
            return false;
        }
        if leave && self.mask_draw.is_none() {
            self.set_tool(Tool::Edit);
            return true;
        }
        let brush = self.mask_draw == Some(MaskDraw::Brush);
        match (combo.key.as_str(), combo.shift) {
            ("[", false) if brush => self.brush.size = (self.brush.size / 1.15).max(0.002),
            ("]", false) if brush => self.brush.size = (self.brush.size * 1.15).min(0.5),
            ("[", true) if brush => self.brush.feather = (self.brush.feather - 0.1).max(0.0),
            ("]", true) if brush => self.brush.feather = (self.brush.feather + 0.1).min(1.0),
            ("escape", false) | ("return", false) if self.mask_draw.is_some() => self.mask_draw = None,
            ("'", false) => {
                let (Some(m), Some(p)) = (self.selected_mask, self.selected_part) else { return false };
                self.update_part(m, p, None, |x| x.invert = !x.invert);
            }
            ("delete", false) => {
                let (Some(m), Some(p)) = (self.selected_mask, self.selected_part) else { return false };
                self.delete_part(m, p);
            }
            _ => return false,
        }
        true
    }

    /// Links a layer to a mask (None = whole image).
    pub fn set_layer_mask(&mut self, layer: Uuid, mask: Option<MaskRef>) { self.update_layer(layer, None, |n| n.mask = mask); }
    pub fn reset_crop(&mut self) { self.crop_draft = CropRect::default(); }

    // MARK: - Messages

    pub fn flash(&mut self, s: &str) { self.toast = Some((s.to_string(), Instant::now())); }

    pub fn alert(&mut self, title: &str, msg: &str) { self.dialog = Some(Dialog::Alert { title: title.into(), message: msg.into() }); }

    // MARK: - Actions (menus, shortcuts, command palette)

    pub fn perform(&mut self, a: Action, ctx: &egui::Context) {
        let has = self.has_image();
        match a {
            Action::Open => self.open_panel(),
            Action::Save => self.save(false),
            Action::SaveAs => self.save(true),
            Action::Export => {
                if has {
                    self.show_export = true;
                }
            }
            Action::CopyResult => self.copy_result(),
            Action::Paste => self.paste_image(),
            Action::Undo => self.undo(),
            Action::Redo => self.redo(),
            Action::CommandPalette => {
                if has {
                    self.show_command_palette = !self.show_command_palette;
                }
            }
            Action::AddEffect => {
                if has {
                    self.show_effect_browser = !self.show_effect_browser;
                    if !self.show_effect_browser {
                        self.set_preview_layers(None);
                    }
                }
            }
            Action::BeforeAfter => {
                if has {
                    self.show_original = !self.show_original;
                    if self.show_original {
                        self.compare_split = false;
                    }
                }
            }
            Action::Crop => self.toggle_tool(Tool::Crop),
            Action::ToolEdit => self.set_tool(Tool::Edit),
            Action::ToolCanvas => self.toggle_tool(Tool::Canvas),
            Action::ToolMasks => self.toggle_tool(Tool::Masks),
            Action::Group => self.group_selected(),
            Action::Ungroup => self.ungroup_selected(),
            Action::Duplicate => self.duplicate_selected(),
            Action::Delete => self.delete_selected(),
            Action::ToggleVisibility => self.toggle_selected_visibility(),
            Action::MoveUp => self.nudge_selected(-1),
            Action::MoveDown => self.nudge_selected(1),
            Action::ZoomIn => self.zoom_in(),
            Action::ZoomOut => self.zoom_out(),
            // Z toggles between fit and 100% (Lightroom)
            Action::ZoomFit => {
                if self.zoom().is_none() {
                    self.zoom_actual()
                } else {
                    self.set_zoom(None)
                }
            }
            Action::Zoom100 => self.zoom_actual(),
            Action::Present => self.toggle_presentation(ctx),
            Action::PrevImage => self.activate_next(-1),
            Action::NextImage => self.activate_next(1),
            Action::Filmstrip => settings::update(|s| s.show_filmstrip = !s.show_filmstrip),
            Action::SyncSettings => self.show_sync = self.strip_selection.len() > 1,
            Action::CloseImage => {
                if let Some(id) = self.active_id {
                    self.request_close([id].into_iter().collect());
                }
            }
            Action::Compare => {
                if has {
                    self.compare_split = !self.compare_split;
                    if self.compare_split {
                        self.show_original = false;
                    }
                }
            }
            Action::Randomize => {
                if let (Some(id), false) = (self.selected_id, self.is_multi_selection()) {
                    self.randomize_params(id);
                }
            }
            Action::CopyLayers => self.copy_layers(),
            Action::PasteLayers => self.paste_layers(),
            Action::ShowLayers => {
                self.show_effect_browser = false;
                self.set_preview_layers(None);
            }
            Action::ShowPresets => {
                if has {
                    self.set_show_presets(!self.show_preset_browser);
                }
            }
            Action::CopySettings => {
                if has {
                    self.copy_settings();
                }
            }
            Action::PasteSettings => {
                let mut ids = self.strip_selection.clone();
                ids.extend(self.active_id);
                self.paste_settings(&ids);
            }
            Action::ResetSettings => {
                let mut ids = self.strip_selection.clone();
                ids.extend(self.active_id);
                self.reset_settings(&ids);
            }
            Action::TogglePanels => {
                if self.hide_all_panels {
                    self.hide_all_panels = false;
                    self.hide_side_panels = false;
                } else {
                    self.hide_side_panels = !self.hide_side_panels;
                }
            }
            Action::ToggleAllPanels => {
                self.hide_all_panels = !self.hide_all_panels;
                self.hide_side_panels = self.hide_all_panels;
            }
            Action::ShowClipping => {
                self.show_clipping = !self.show_clipping;
                self.request_render();
            }
            Action::ToggleHistogram => {
                settings::update(|s| s.histogram_open = !s.histogram_open);
                self.request_render();
            }
            Action::SelectAll => self.select_all_layers(),
            Action::MaskBrush => self.start_mask_draw(Some(MaskDraw::Brush)),
            Action::MaskLinear => self.start_mask_draw(Some(MaskDraw::Linear)),
            Action::MaskRadial => self.start_mask_draw(Some(MaskDraw::Radial)),
            Action::MaskSubject => {
                if has {
                    let into = self.mask_draw_into.take();
                    self.add_mask_part(MaskShape::subject_default(), into);
                }
            }
            Action::MaskBackground => {
                if has {
                    let into = self.mask_draw_into.take();
                    self.add_mask_part(MaskShape::background_default(), into);
                }
            }
            Action::Deselect => {
                self.set_selected(None);
                self.selected_ids.clear();
                self.strip_selection = self.active_id.into_iter().collect();
            }
        }
    }

    pub fn set_show_presets(&mut self, on: bool) {
        self.show_preset_browser = on;
        if on {
            self.show_effect_browser = false;
        }
        self.set_preview_layers(None);
        settings::update(|s| s.presets_open = on);
    }

    pub fn ctx(&self) -> &egui::Context { &self.ctx }
}

pub const LAYERS_CLIP_PREFIX: &str = "EFFECTOR-LAYERS:";

pub fn session_dir() -> PathBuf {
    let d = settings::support_dir().join("Session");
    let _ = std::fs::create_dir_all(&d);
    d
}

fn rand_seed() -> u64 {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(42);
    t ^ 0x5DEECE66D
}
