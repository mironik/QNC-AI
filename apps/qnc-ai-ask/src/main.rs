//! QNC-AI question (user 2026-11-15: "prozor koji daje obavijest da je baza spremna izgleda kao
//! sistemski i ne dolazi na vrh, pa se ne vidi"): a question of the tray in QNC's look, above every
//! window and asking for attention until it is answered. The tray starts it with the title, the
//! text and the buttons (`yes,no` or `yes,no,cancel`) and reads the answer from its stdout: `yes`,
//! `no` or `cancel` (closing the window is `cancel`). Enter is `--enter` (else the first button),
//! Esc the last.

#![windows_subsystem = "windows"]

use eframe::egui::{self, RichText};

struct Ask {
    title: String,
    text: String,
    buttons: Vec<(String, String)>,
    /// The answer Enter gives.
    enter: String,
    theme: Option<qnc_ingest_layout::theme::Theme>,
    answered: bool,
}

impl Ask {
    fn answer(&mut self, ctx: &egui::Context, answer: &str) {
        if !self.answered {
            println!("{answer}");
            self.answered = true;
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

impl eframe::App for Ask {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let (bg, text, font) = self.theme.as_ref().map_or((egui::Color32::from_gray(24), egui::Color32::from_gray(230), 14.0), |theme| (theme.bg, theme.text, theme.font_ui));
        let keys = ctx.input(|input| (input.key_pressed(egui::Key::Enter), input.key_pressed(egui::Key::Escape)));
        if keys.0 {
            let enter = self.enter.clone();
            return self.answer(ctx, &enter);
        }
        if keys.1 || ctx.input(|input| input.viewport().close_requested()) {
            let last = if keys.1 { self.buttons.last().map(|(answer, _)| answer.clone()).unwrap_or_else(|| "cancel".into()) } else { "cancel".into() };
            return self.answer(ctx, &last);
        }
        let mut chosen = None;
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(bg).inner_margin(20.0)).show(ctx, |ui| {
            ui.label(RichText::new(&self.title).size(font + 4.0).color(text).strong());
            ui.add_space(10.0);
            ui.label(RichText::new(&self.text).size(font).color(text));
            ui.add_space(18.0);
            ui.horizontal(|ui| {
                for (answer, label) in &self.buttons {
                    if ui.add(egui::Button::new(RichText::new(label).size(font)).min_size(egui::vec2(110.0, 32.0))).clicked() {
                        chosen = Some(answer.clone());
                    }
                }
            });
        });
        if let Some(answer) = chosen {
            self.answer(ctx, &answer);
        }
    }
}

/// The value after `--name` in the arguments.
fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|arg| arg == name).and_then(|at| args.get(at + 1)).cloned()
}

fn main() -> eframe::Result<()> {
    qnc_dev_diagnostics::log_panics("qnc-ai-ask");
    let args: Vec<String> = std::env::args().collect();
    let title = arg(&args, "--title").unwrap_or_else(|| "QNC AI".into());
    let text = arg(&args, "--text").unwrap_or_default();
    // `answer=label` pairs, in order: Enter is the first, Esc the last.
    let buttons: Vec<(String, String)> = arg(&args, "--buttons")
        .unwrap_or_else(|| "yes=Da,no=Ne".into())
        .split(',')
        .filter_map(|pair| pair.split_once('=').map(|(answer, label)| (answer.to_string(), label.to_string())))
        .collect();
    let enter = arg(&args, "--enter").or_else(|| buttons.first().map(|(answer, _)| answer.clone())).unwrap_or_else(|| "yes".into());
    let theme = qnc_ingest_layout::IngestContracts::load().ok().map(|contracts| qnc_ingest_layout::theme::Theme::from_contract(&contracts.shell));
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_title(&title)
            .with_inner_size([560.0, 240.0])
            .with_resizable(false)
            .with_always_on_top()
            .with_active(true),
        persist_window: false,
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "QNC AI question",
        options,
        Box::new(move |cc| {
            if let Some(theme) = &theme {
                qnc_ingest_layout::theme::apply(&cc.egui_ctx, theme);
            }
            cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
            Ok(Box::new(Ask { title, text, buttons, enter, theme, answered: false }))
        }),
    )
}
