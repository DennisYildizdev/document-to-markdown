use crate::theme::{self, Palette, SIZE_BODY, SIZE_READING, SIZE_SMALL, SIZE_TITLE};
use document_to_markdown::{
    AppError, ConversionResult, InputSummary, Registry, SUPPORTED_EXTENSIONS,
};
use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Key, KeyboardShortcut, Layout, Margin, Modifiers,
    RichText, Sense, Stroke,
};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::{
    path::PathBuf,
    sync::{Arc, mpsc},
    thread,
};

const PREVIEW_LIMIT_BYTES: usize = 256 * 1024;
const CLIPBOARD_LIMIT_BYTES: usize = 1024 * 1024;
const NOTICE_SECONDS: f64 = 6.0;
const READING_WIDTH: f32 = 720.0;

const ADD: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);
const CONVERT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Enter);
const EXPORT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);

type Done = (String, Result<ConversionResult, AppError>);

enum Status {
    Pending,
    Converting,
    Ready(ConversionResult),
    Failed(AppError),
}

struct Item {
    summary: InputSummary,
    status: Status,
}

#[derive(PartialEq, Clone, Copy)]
enum View {
    Rendered,
    Source,
}

struct Notice {
    text: String,
    is_error: bool,
    shown_at: f64,
}

pub struct App {
    registry: Arc<Registry>,
    items: Vec<Item>,
    selected: Option<usize>,
    view: View,
    markdown_cache: CommonMarkCache,
    notice: Option<Notice>,
    last_export_dir: Option<PathBuf>,
    jobs: mpsc::Sender<String>,
    done: mpsc::Receiver<Done>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install(&cc.egui_ctx);

        // One worker keeps the window responsive and runs one parser at a time.
        let registry = Arc::new(Registry::default());
        let (jobs, job_rx) = mpsc::channel::<String>();
        let (done_tx, done) = mpsc::channel::<Done>();
        let worker_registry = Arc::clone(&registry);
        let ctx = cc.egui_ctx.clone();
        thread::spawn(move || {
            for input_id in job_rx {
                let result = worker_registry.convert(&input_id);
                if done_tx.send((input_id, result)).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });

        let mut app = Self {
            registry,
            items: Vec::new(),
            selected: None,
            view: View::Rendered,
            markdown_cache: CommonMarkCache::default(),
            notice: None,
            last_export_dir: None,
            jobs,
            done,
        };
        // Files passed on the command line (Explorer "Open with", or dropped on the .exe).
        let paths: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
        if !paths.is_empty() {
            app.add_paths(&cc.egui_ctx, paths);
        }
        app
    }

    fn selected_item(&self) -> Option<&Item> {
        self.selected.and_then(|index| self.items.get(index))
    }

    fn selected_result(&self) -> Option<&ConversionResult> {
        match self.selected_item() {
            Some(Item {
                status: Status::Ready(result),
                ..
            }) => Some(result),
            _ => None,
        }
    }

    fn convertible_count(&self) -> usize {
        self.items
            .iter()
            .filter(|item| matches!(item.status, Status::Pending | Status::Failed(_)))
            .count()
    }

    fn notify(&mut self, ctx: &egui::Context, text: impl Into<String>, is_error: bool) {
        self.notice = Some(Notice {
            text: text.into(),
            is_error,
            shown_at: ctx.input(|input| input.time),
        });
    }

    // Actions

    fn pick_files(&mut self, ctx: &egui::Context) {
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter("Documents", SUPPORTED_EXTENSIONS)
            .pick_files()
        {
            self.add_paths(ctx, paths);
        }
    }

    fn add_paths(&mut self, ctx: &egui::Context, paths: Vec<PathBuf>) {
        let mut skipped = Vec::new();
        for path in paths {
            let name = path
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
            match self.registry.register_paths(vec![path]) {
                Ok(summaries) => {
                    for summary in summaries {
                        let existing = self
                            .items
                            .iter()
                            .position(|item| item.summary.input_id == summary.input_id);
                        self.selected = Some(existing.unwrap_or_else(|| {
                            self.items.push(Item {
                                summary,
                                status: Status::Pending,
                            });
                            self.items.len() - 1
                        }));
                    }
                }
                Err(error) => skipped.push(format!("{name}: {}", error.message)),
            }
        }
        match skipped.len() {
            0 => {}
            1 => self.notify(ctx, skipped.remove(0), true),
            count => self.notify(
                ctx,
                format!("{count} files were not added. {}", skipped[0]),
                true,
            ),
        }
    }

    fn convert_all(&mut self) {
        for item in &mut self.items {
            if matches!(item.status, Status::Pending | Status::Failed(_))
                && self.jobs.send(item.summary.input_id.clone()).is_ok()
            {
                item.status = Status::Converting;
            }
        }
    }

    fn convert_one(&mut self, index: usize) {
        if let Some(item) = self.items.get_mut(index)
            && self.jobs.send(item.summary.input_id.clone()).is_ok()
        {
            item.status = Status::Converting;
        }
    }

    fn export_selected(&mut self, ctx: &egui::Context) {
        let Some(result_id) = self
            .selected_result()
            .map(|result| result.result_id.clone())
        else {
            return;
        };
        let mut dialog = rfd::FileDialog::new().set_title("Export Markdown to folder");
        if let Some(dir) = &self.last_export_dir {
            dialog = dialog.set_directory(dir);
        }
        let Some(directory) = dialog.pick_folder() else {
            return;
        };
        match self.registry.export_to_directory(&result_id, &directory) {
            Ok(path) => {
                let name = path
                    .file_name()
                    .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
                self.notify(
                    ctx,
                    format!("Saved {name} to {}", directory.display()),
                    false,
                );
                self.last_export_dir = Some(directory);
            }
            Err(error) => self.notify(ctx, format!("Export failed. {}", error.message), true),
        }
    }

    fn copy_selected(&mut self, ctx: &egui::Context) {
        let Some(markdown) = self.selected_result().map(|result| result.markdown.clone()) else {
            return;
        };
        if markdown.len() > CLIPBOARD_LIMIT_BYTES {
            self.notify(
                ctx,
                "This result is larger than 1 MiB. Export it instead of copying.",
                true,
            );
        } else {
            ctx.copy_text(markdown);
            self.notify(ctx, "Markdown copied.", false);
        }
    }

    fn remove_selected(&mut self) {
        if let Some(index) = self.selected
            && index < self.items.len()
            && !matches!(self.items[index].status, Status::Converting)
        {
            self.items.remove(index);
            self.selected = if self.items.is_empty() {
                None
            } else {
                Some(index.min(self.items.len() - 1))
            };
        }
    }

    fn move_selection(&mut self, delta: isize) {
        if self.items.is_empty() {
            return;
        }
        let last = self.items.len() as isize - 1;
        let next = self.selected.map_or(0, |index| index as isize + delta);
        self.selected = Some(next.clamp(0, last) as usize);
    }

    // Per-frame bookkeeping

    fn receive_results(&mut self) {
        while let Ok((input_id, result)) = self.done.try_recv() {
            if let Some(item) = self
                .items
                .iter_mut()
                .find(|item| item.summary.input_id == input_id)
            {
                item.status = match result {
                    Ok(result) => Status::Ready(result),
                    Err(error) => Status::Failed(error),
                };
            }
        }
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .map(|file| file.path().to_path_buf())
                .filter(|path| !path.as_os_str().is_empty())
                .collect()
        });
        if !dropped.is_empty() {
            self.add_paths(ctx, dropped);
        }

        let text_focused = ctx.memory(|memory| memory.focused().is_some());
        let (add, convert, export, up, down, delete, escape) = ctx.input_mut(|input| {
            (
                input.consume_shortcut(&ADD),
                input.consume_shortcut(&CONVERT),
                input.consume_shortcut(&EXPORT),
                !text_focused && input.consume_key(Modifiers::NONE, Key::ArrowUp),
                !text_focused && input.consume_key(Modifiers::NONE, Key::ArrowDown),
                !text_focused && input.consume_key(Modifiers::NONE, Key::Delete),
                input.consume_key(Modifiers::NONE, Key::Escape),
            )
        });
        if add {
            self.pick_files(ctx);
        }
        if convert {
            self.convert_all();
        }
        if export {
            self.export_selected(ctx);
        }
        if up {
            self.move_selection(-1);
        }
        if down {
            self.move_selection(1);
        }
        if delete {
            self.remove_selected();
        }
        if escape {
            self.notice = None;
        }

        if let Some(notice) = &self.notice
            && !notice.is_error
        {
            let left = NOTICE_SECONDS - (ctx.input(|input| input.time) - notice.shown_at);
            if left <= 0.0 {
                self.notice = None;
            } else {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(left));
            }
        }
    }

    // Layout

    fn toolbar(&mut self, ui: &mut egui::Ui, palette: &Palette) {
        let ctx = ui.ctx().clone();
        // The one primary button is whatever the next step is.
        let convertible = self.convertible_count();
        let ready = self.selected_result().is_some();
        let primary = if self.items.is_empty() {
            0
        } else if convertible > 0 {
            1
        } else if ready {
            2
        } else {
            0
        };

        // Input actions on the left, output actions on the right.
        ui.horizontal_centered(|ui| {
            if button(ui, palette, "Add files…", primary == 0, true)
                .on_hover_text("Ctrl+O")
                .clicked()
            {
                self.pick_files(&ctx);
            }
            let convert_label = if convertible > 1 {
                format!("Convert {convertible}")
            } else {
                "Convert".to_owned()
            };
            if button(ui, palette, &convert_label, primary == 1, convertible > 0)
                .on_hover_text("Convert every document that is not converted yet (Ctrl+Enter)")
                .clicked()
            {
                self.convert_all();
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if button(ui, palette, "Export…", primary == 2, ready)
                    .on_hover_text("Save the selected result as .md (Ctrl+S)")
                    .clicked()
                {
                    self.export_selected(&ctx);
                }
                if button(ui, palette, "Copy", false, ready)
                    .on_hover_text("Copy the selected Markdown")
                    .clicked()
                {
                    self.copy_selected(&ctx);
                }
            });
        });
    }

    fn status_bar(&self, ui: &mut egui::Ui, palette: &Palette) {
        let converted = self
            .items
            .iter()
            .filter(|item| matches!(item.status, Status::Ready(_)))
            .count();
        let failed = self
            .items
            .iter()
            .filter(|item| matches!(item.status, Status::Failed(_)))
            .count();
        let converting = self
            .items
            .iter()
            .filter(|item| matches!(item.status, Status::Converting))
            .count();

        let mut parts = vec![match self.items.len() {
            1 => "1 document".to_owned(),
            count => format!("{count} documents"),
        }];
        if converted > 0 {
            parts.push(format!("{converted} converted"));
        }
        if converting > 0 {
            parts.push(format!("{converting} converting"));
        }
        if failed > 0 {
            parts.push(format!("{failed} failed"));
        }

        ui.horizontal_centered(|ui| {
            ui.label(small(&parts.join("  ·  "), palette.secondary));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(small("Processed on this computer", palette.secondary));
            });
        });
    }

    fn queue(&mut self, ui: &mut egui::Ui, palette: &Palette) {
        ui.add_space(16.0);
        ui.label(caps("Documents", palette.secondary));
        ui.add_space(4.0);

        if self.items.is_empty() {
            ui.add_space(8.0);
            ui.label(RichText::new("No documents yet").color(palette.text));
            ui.label(small(
                "Drop files here or press Ctrl+O.\nTXT · MD · CSV · TSV · JSON · XML · PDF",
                palette.secondary,
            ));
            return;
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut clicked = None;
                let mut remove = None;
                for (index, item) in self.items.iter().enumerate() {
                    let response = queue_row(ui, palette, item, self.selected == Some(index));
                    if response.clicked() {
                        clicked = Some(index);
                    }
                    response.context_menu(|ui| {
                        if ui.button("Remove from list").clicked() {
                            remove = Some(index);
                        }
                    });
                }
                if let Some(index) = clicked {
                    self.selected = Some(index);
                }
                if let Some(index) = remove {
                    self.selected = Some(index);
                    self.remove_selected();
                }
                ui.add_space(16.0);
            });
    }

    fn content(&mut self, ui: &mut egui::Ui, palette: &Palette) {
        if let Some(notice) = &self.notice {
            let color = if notice.is_error {
                palette.danger
            } else {
                palette.secondary
            };
            let text = notice.text.clone();
            let mut dismiss = false;
            ui.horizontal(|ui| {
                ui.add(egui::Label::new(RichText::new(text).color(color)).truncate());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(egui::Button::new(small("Dismiss", palette.secondary)).frame(false))
                        .on_hover_text("Esc")
                        .clicked()
                    {
                        dismiss = true;
                    }
                });
            });
            if dismiss {
                self.notice = None;
            }
            ui.add_space(16.0);
        }

        let Some(index) = self.selected.filter(|index| *index < self.items.len()) else {
            empty_state(ui, palette, self.items.is_empty());
            if self.items.is_empty() {
                ui.add_space(24.0);
                let ctx = ui.ctx().clone();
                if button(ui, palette, "Add files…", true, true).clicked() {
                    self.pick_files(&ctx);
                }
            }
            return;
        };

        let item = &self.items[index];
        let summary = item.summary.clone();

        // Header: the file name is the largest text on screen.
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(&summary.name)
                            .font(theme::semibold(SIZE_TITLE))
                            .color(palette.text),
                    )
                    .truncate(),
                );
                ui.label(
                    RichText::new(format!(
                        "{}  ·  {}",
                        theme::format_label(&summary.format),
                        format_bytes(summary.size)
                    ))
                    .color(palette.secondary),
                );
            });
            if matches!(item.status, Status::Ready(_)) {
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    ui.add_space(0.0);
                    segmented(ui, palette, &mut self.view);
                });
            }
        });
        ui.add_space(24.0);

        match &self.items[index].status {
            Status::Pending => {
                ui.label(
                    RichText::new("Not converted yet")
                        .size(SIZE_READING)
                        .color(palette.text),
                );
                ui.label(RichText::new("Press Convert or Ctrl+Enter.").color(palette.secondary));
            }
            Status::Converting => {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Spinner::new()
                            .size(SIZE_READING)
                            .color(palette.secondary),
                    );
                    ui.label(
                        RichText::new("Converting…")
                            .size(SIZE_READING)
                            .color(palette.text),
                    );
                });
            }
            Status::Failed(error) => {
                let code = error.code;
                let message = error.message.clone();
                ui.label(
                    RichText::new(failure_title(code))
                        .font(theme::semibold(SIZE_READING))
                        .color(palette.danger),
                );
                ui.add(egui::Label::new(RichText::new(message).color(palette.text)).wrap());
                ui.label(small(code, palette.secondary));
                ui.add_space(16.0);
                if button(ui, palette, "Try again", false, true).clicked() {
                    self.convert_one(index);
                }
            }
            Status::Ready(result) => {
                for diagnostic in &result.diagnostics {
                    let (label, color) = match diagnostic.severity {
                        conversion_core::DiagnosticSeverity::Error => ("Error", palette.danger),
                        conversion_core::DiagnosticSeverity::Warning => {
                            ("Warning", palette.warning)
                        }
                        conversion_core::DiagnosticSeverity::Info => ("Note", palette.secondary),
                    };
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(label)
                                .font(theme::semibold(SIZE_SMALL))
                                .color(color),
                        );
                        ui.label(small(&diagnostic.message, palette.text))
                            .on_hover_text(&diagnostic.code);
                    });
                }

                let (shown, truncated) = preview_slice(&result.markdown);
                if truncated {
                    ui.label(small(
                        "Preview shows the first 256 KiB. Export contains the full document.",
                        palette.secondary,
                    ));
                }
                if !result.diagnostics.is_empty() || truncated {
                    ui.add_space(8.0);
                }
                ui.separator();
                ui.add_space(8.0);

                let view = self.view;
                let id = summary.input_id.clone();
                egui::ScrollArea::vertical()
                    .id_salt(("preview", &id, view == View::Source))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_max_width(READING_WIDTH.min(ui.available_width()));
                        ui.add_space(8.0);
                        match view {
                            View::Rendered => {
                                ui.style_mut().text_styles.insert(
                                    egui::TextStyle::Body,
                                    egui::FontId::proportional(SIZE_READING),
                                );
                                ui.spacing_mut().item_spacing.y = 10.0;
                                let secondary = palette.secondary;
                                let html = move |ui: &mut egui::Ui, html: &str| {
                                    render_html(ui, html, secondary)
                                };
                                CommonMarkViewer::new().render_html_fn(Some(&html)).show(
                                    ui,
                                    &mut self.markdown_cache,
                                    shown,
                                );
                            }
                            View::Source => {
                                let mut text = shown;
                                ui.add(
                                    egui::TextEdit::multiline(&mut text)
                                        .code_editor()
                                        .frame(Frame::NONE)
                                        .desired_width(f32::INFINITY),
                                );
                            }
                        }
                        ui.add_space(32.0);
                    });
            }
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.receive_results();
        self.handle_input(&ctx);
        let palette = Palette::of(&ctx);

        egui::Panel::top("toolbar")
            .exact_size(52.0)
            .frame(
                Frame::new()
                    .fill(palette.background)
                    .inner_margin(Margin::symmetric(16, 0)),
            )
            .show(ui, |ui| self.toolbar(ui, &palette));

        egui::Panel::bottom("status")
            .exact_size(28.0)
            .frame(
                Frame::new()
                    .fill(palette.sidebar)
                    .inner_margin(Margin::symmetric(16, 0)),
            )
            .show(ui, |ui| self.status_bar(ui, &palette));

        egui::Panel::left("queue")
            .resizable(true)
            .default_size(280.0)
            .min_size(220.0)
            .max_size(420.0)
            .frame(
                Frame::new()
                    .fill(palette.sidebar)
                    .inner_margin(Margin::symmetric(12, 0)),
            )
            .show(ui, |ui| self.queue(ui, &palette));

        egui::CentralPanel::default()
            .frame(Frame::new().fill(palette.background).inner_margin(Margin {
                left: 40,
                right: 40,
                top: 32,
                bottom: 0,
            }))
            .show(ui, |ui| self.content(ui, &palette));

        drop_overlay(&ctx, &palette);
    }
}

// Widgets

fn button(
    ui: &mut egui::Ui,
    palette: &Palette,
    text: &str,
    primary: bool,
    enabled: bool,
) -> egui::Response {
    ui.scope(|ui| {
        if primary {
            let widgets = &mut ui.visuals_mut().widgets;
            widgets.inactive.weak_bg_fill = palette.accent;
            widgets.hovered.weak_bg_fill = palette.accent_hover;
            widgets.active.weak_bg_fill = palette.accent_hover;
            for state in [
                &mut widgets.inactive,
                &mut widgets.hovered,
                &mut widgets.active,
            ] {
                state.fg_stroke = Stroke::new(1.0, palette.on_accent);
            }
        }
        let text = if primary {
            RichText::new(text).font(theme::semibold(SIZE_BODY))
        } else {
            RichText::new(text)
        };
        ui.add_enabled(
            enabled,
            egui::Button::new(text).min_size(egui::vec2(0.0, 30.0)),
        )
    })
    .inner
}

fn segmented(ui: &mut egui::Ui, palette: &Palette, view: &mut View) {
    Frame::new()
        .fill(palette.surface)
        .corner_radius(CornerRadius::same(theme::RADIUS))
        .inner_margin(Margin::same(2))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            // Right-to-left parent: add in reverse so the order reads Rendered | Source.
            for (option, label) in [(View::Source, "Source"), (View::Rendered, "Rendered")] {
                let selected = *view == option;
                let fill = if selected {
                    palette.background
                } else {
                    Color32::TRANSPARENT
                };
                let text = RichText::new(label).size(SIZE_SMALL).color(if selected {
                    palette.text
                } else {
                    palette.secondary
                });
                let response = ui.add(
                    egui::Button::new(text)
                        .fill(fill)
                        .corner_radius(CornerRadius::same(6))
                        .min_size(egui::vec2(72.0, 24.0)),
                );
                if response.clicked() {
                    *view = option;
                }
            }
        });
}

fn queue_row(ui: &mut egui::Ui, palette: &Palette, item: &Item, selected: bool) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 54.0), Sense::click());
    let fill = if selected {
        palette.row_selected
    } else if response.hovered() {
        palette.row_selected.gamma_multiply(0.5)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(theme::RADIUS), fill);
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(theme::RADIUS),
            Stroke::new(2.0, palette.accent),
            egui::StrokeKind::Inside,
        );
    }

    let dot = egui::pos2(rect.left() + 14.0, rect.top() + 18.0);
    ui.painter()
        .circle_filled(dot, 4.0, palette.format(&item.summary.format));

    let (status, status_color) = match &item.status {
        Status::Pending => ("Not converted", palette.secondary),
        Status::Converting => ("Converting…", palette.secondary),
        Status::Ready(result) if !result.diagnostics.is_empty() => {
            ("Converted with notes", palette.secondary)
        }
        Status::Ready(_) => ("Converted", palette.secondary),
        Status::Failed(_) => ("Failed", palette.danger),
    };

    // Painted rather than laid out as widgets so long names never widen the sidebar.
    let left = rect.left() + 28.0;
    let width = rect.right() - 8.0 - left;
    let name = one_line(
        &[(
            &item.summary.name,
            egui::FontId::proportional(SIZE_BODY),
            palette.text,
        )],
        width,
    );
    let meta_text = format!(
        "{}  ·  {}  ·  ",
        theme::format_label(&item.summary.format),
        format_bytes(item.summary.size)
    );
    let small_font = egui::FontId::proportional(SIZE_SMALL);
    let meta = one_line(
        &[
            (&meta_text, small_font.clone(), palette.secondary),
            (status, small_font, status_color),
        ],
        width,
    );
    let painter = ui.painter();
    let name = painter.layout_job(name);
    let meta = painter.layout_job(meta);
    let top = rect.top() + (rect.height() - name.size().y - 2.0 - meta.size().y) / 2.0;
    painter.galley(egui::pos2(left, top), name.clone(), palette.text);
    painter.galley(
        egui::pos2(left, top + name.size().y + 2.0),
        meta,
        palette.secondary,
    );

    response.on_hover_text(&item.summary.name)
}

fn empty_state(ui: &mut egui::Ui, palette: &Palette, nothing_added: bool) {
    let (title, body) = if nothing_added {
        (
            "Drop a document to begin.",
            "Text, Markdown, CSV, TSV, JSON, XML, and PDF with selectable text. \
             Everything is converted on this computer.",
        )
    } else {
        ("Select a document.", "Its Markdown appears here.")
    };
    ui.add_space(64.0);
    ui.label(
        RichText::new(title)
            .font(theme::semibold(SIZE_TITLE))
            .color(palette.text),
    );
    ui.add_space(4.0);
    ui.scope(|ui| {
        ui.set_max_width(480.0);
        ui.add(
            egui::Label::new(
                RichText::new(body)
                    .size(SIZE_READING)
                    .color(palette.secondary),
            )
            .wrap(),
        );
    });
}

fn drop_overlay(ctx: &egui::Context, palette: &Palette) {
    let hovering = ctx.input(|input| !input.raw.hovered_files.is_empty());
    if !hovering {
        return;
    }
    let screen = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::unique("drop-overlay"),
    ));
    painter.rect_filled(
        screen,
        CornerRadius::ZERO,
        palette.background.gamma_multiply(0.92),
    );
    painter.rect_stroke(
        screen.shrink(12.0),
        CornerRadius::same(12),
        Stroke::new(2.0, palette.accent),
        egui::StrokeKind::Inside,
    );
    painter.text(
        screen.center(),
        egui::Align2::CENTER_CENTER,
        "Drop to add",
        theme::semibold(theme::SIZE_SECTION),
        palette.text,
    );
}

// Text helpers

fn small(text: &str, color: Color32) -> RichText {
    RichText::new(text).size(SIZE_SMALL).color(color)
}

fn caps(text: &str, color: Color32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        &text.to_uppercase(),
        0.0,
        egui::TextFormat {
            font_id: theme::semibold(SIZE_SMALL),
            color,
            extra_letter_spacing: 0.6,
            ..Default::default()
        },
    );
    job
}

fn failure_title(code: &str) -> &'static str {
    match code {
        "OCR_REQUIRED" => "This PDF has no selectable text",
        "PASSWORD_PROTECTED" => "This document is password protected",
        "INVALID_ENCODING" => "This file is not valid UTF-8",
        "MALFORMED_DOCUMENT" => "This document could not be read",
        "ACCESS_DENIED" => "The file could not be opened",
        _ => "Conversion failed",
    }
}

fn preview_slice(markdown: &str) -> (&str, bool) {
    if markdown.len() <= PREVIEW_LIMIT_BYTES {
        return (markdown, false);
    }
    let mut end = PREVIEW_LIMIT_BYTES;
    while !markdown.is_char_boundary(end) {
        end -= 1;
    }
    (&markdown[..end], true)
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MiB", bytes as f64 / 1024.0 / 1024.0)
    }
}

/// A single line of mixed-color text, cut with an ellipsis at `width`.
fn one_line(parts: &[(&str, egui::FontId, Color32)], width: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    for (text, font_id, color) in parts {
        job.append(
            text,
            0.0,
            egui::TextFormat {
                font_id: font_id.clone(),
                color: *color,
                ..Default::default()
            },
        );
    }
    job.wrap = egui::text::TextWrapping::truncate_at_width(width);
    job
}

/// Raw HTML is never interpreted. Page markers from the PDF converter become a
/// quiet label; anything else is shown as literal text.
fn render_html(ui: &mut egui::Ui, html: &str, color: Color32) {
    let page = html
        .trim()
        .strip_prefix("<!-- page ")
        .and_then(|rest| rest.strip_suffix(" -->"));
    match page {
        Some(number) => {
            ui.add_space(8.0);
            ui.label(caps(&format!("Page {number}"), color));
        }
        None => {
            ui.label(RichText::new(html.trim_end()).monospace().color(color));
        }
    }
}
