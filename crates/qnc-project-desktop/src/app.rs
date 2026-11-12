use eframe::egui;
use qnc_project_blocks::{layout_contract::AppContracts, theme::Theme};
use qnc_project_session::ProjectSession;

/// Project on the desktop board: the contracts, the session and its blocks. The form
/// only passes what the blocks ask for to the session.
pub struct ProjectApp {
    contracts: AppContracts,
    session: ProjectSession,
    /// QNC-AI: only the settings (the Settings form the tray opens).
    settings_only: bool,
    /// QNC-AI: only the project list (the Projects form); the project a row opens.
    projects_only: bool,
    opened: Option<String>,
}

impl ProjectApp {
    pub(crate) fn new(contracts: AppContracts, session: ProjectSession) -> Self {
        Self { contracts, session, settings_only: false, projects_only: false, opened: None }
    }

    /// QNC-AI: the project list on its own, only the projects `only`; a row opened is not opened
    /// here but given by [`Self::take_opened`].
    pub fn projects_only(mut self, only: std::collections::BTreeSet<String>) -> Self {
        self.projects_only = true;
        self.session.only = Some(only);
        self
    }

    /// The id of the project a row of the Projects form opened.
    pub fn take_opened(&mut self) -> Option<String> {
        self.opened.take()
    }

    /// QNC-AI: the settings on their own, their advanced part open.
    pub fn settings_only(mut self) -> Self {
        self.settings_only = true;
        self.session.advanced_open = true;
        self
    }

    pub fn footer_status(&self) -> &str {
        self.session.footer_status()
    }

    pub fn take_navigation_trigger(&mut self) -> bool {
        self.session.take_navigation_trigger()
    }

    pub fn navigation_sequence(
        &self,
    ) -> Result<Vec<qnc_project_application::ProjectNavigationStep>, String> {
        self.session.navigation_sequence()
    }

    pub fn show_desktop(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, frame: &mut qnc_board::Frame<'_>) {
        // The registry works in the background: its replies are taken here.
        if self.session.component.applications.poll() | self.session.poll() || self.session.busy() {
            ctx.request_repaint_after(std::time::Duration::from_millis(20));
        }
        if self.projects_only {
            for intent in qnc_project_blocks::show_projects(ctx, ui, &self.contracts, &mut self.session) {
                let row = match intent {
                    qnc_project_session::ProjectIntent::OpenProjectRow { index } => Some(index),
                    qnc_project_session::ProjectIntent::OpenSelectedProject => self.session.selected_project,
                    intent => {
                        self.session.dispatch(intent);
                        None
                    }
                };
                if let Some(project) = row.and_then(|index| self.session.projects.get(index)) {
                    self.opened = Some(project.project_id.clone());
                }
            }
            return;
        }
        let intents = if self.settings_only {
            qnc_project_blocks::show_settings(ctx, ui, &self.contracts, &mut self.session)
        } else {
            qnc_project_blocks::show(ctx, ui, frame, &self.contracts, &mut self.session)
        };
        for intent in intents {
            self.session.dispatch(intent);
        }
    }
}

impl eframe::App for ProjectApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let t = Theme::from_contract(&self.contracts.shell.colors);
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(t.bg))
            .show(ctx, |ui| self.show_desktop(ctx, ui, &mut qnc_board::Frame::bare()));
    }
}
