use eframe::{egui, egui::Context, Frame, Storage as EStorage};
use janki::{
    csv::{read_in, write_out},
    dummy_storage::{DummyStorage, DynStorage},
    game::{default_sag, AnkiGame, GiveFacts},
    item::Fact,
    storage::Storage as JStorage,
};
use std::{fs::File, time::Duration};
use tracing::Level;

pub enum JankiState {
    Testing {
        current_text: String,
        current_fact: Fact,
        was_eligible: bool,
    },
    Tested {
        fact: Fact,
        was_correct: bool,
    },
    AddingNew {
        term: String,
        def: String,
    },
    Viewing {
        show_defs: bool,
        show_only_eligible: bool,
    },
    Csv {
        file_name: String,
        overwrite_existing: bool,
    },
}

pub struct JankiApp {
    app: AnkiGame<DummyStorage, GiveFacts>,
    has_done_initial_read: bool,
    state: JankiState,
}

impl JankiApp {
    pub fn new() -> Self {
        Self {
            app: AnkiGame::new(DummyStorage::default(), default_sag()).unwrap(),
            state: JankiState::Viewing {
                show_defs: false,
                show_only_eligible: true,
            },
            has_done_initial_read: false,
        }
    }
}

impl eframe::App for JankiApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        if self.has_done_initial_read {
            egui::SidePanel::left("left_side").show(ctx, |ui| {
                if ui.button("New Test").clicked() {
                    if let Some((f, was_eligible)) = self.app.get_new_fact() {
                        self.state = JankiState::Testing {
                            current_text: String::default(),
                            current_fact: f,
                            was_eligible,
                        };
                    }
                } else if ui.button("Add More").clicked() {
                    self.state = JankiState::AddingNew {
                        term: String::default(),
                        def: String::default(),
                    }
                } else if ui.button("View").clicked() {
                    self.state = JankiState::Viewing {
                        show_defs: false,
                        show_only_eligible: true,
                    }
                } else if ui.button("CSV Utilities").clicked() {
                    self.state = JankiState::Csv {
                        file_name: "./data.csv".into(),
                        overwrite_existing: false,
                    };
                }

                ui.separator();

                if let JankiState::Viewing {
                    show_defs,
                    show_only_eligible,
                } = &mut self.state
                {
                    ui.checkbox(show_defs, "Show definitions: ");
                    ui.checkbox(show_only_eligible, "Show only eligible: ");
                    ui.separator();
                }

                ui.label(format!(
                    "Only {} Facts remaining this session!",
                    self.app.get_eligible_no(),
                ));
            });

            egui::CentralPanel::default().show(ctx, |ui| {
                match &mut self.state {
                    JankiState::Testing {
                        current_fact,
                        current_text,
                        was_eligible,
                    } => {
                        if *was_eligible {
                            ui.label("Testing");
                        } else {
                            ui.label("EVEN MORE TESTING!");
                        }
                        ui.separator();

                        ui.label(format!("The term is: {}", current_fact.term));

                        ui.horizontal(|ui| {
                            ui.label("Please enter the definition: ");
                            ui.text_edit_singleline(current_text);
                        });

                        ui.separator();

                        if ui.button("Submit!").clicked() {
                            let was_correct = current_text.trim() == current_fact.definition;
                            self.app.finish_current_fact(Some(was_correct));

                            self.state = JankiState::Tested {
                                fact: current_fact.clone(),
                                was_correct,
                            };
                        }
                    }
                    JankiState::Tested { fact, was_correct } => {
                        if *was_correct {
                            ui.label("Correct!");
                        } else {
                            ui.label(format!("Wrong - it should've been {:?}", fact.definition));
                        }
                    }
                    JankiState::AddingNew { term, def } => {
                        ui.label("Add New Stuff");
                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.label("Enter a term: ");
                            ui.text_edit_singleline(term);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Enter a definition: ");
                            ui.text_edit_singleline(def);
                        });

                        if ui.button("Submit").clicked() {
                            self.app
                                .add_fact(Fact::new(term.to_string(), def.to_string()));
                            term.clear();
                            def.clear();
                        }
                    }
                    JankiState::Viewing {
                        show_defs,
                        show_only_eligible,
                    } => {
                        let list = if *show_only_eligible {
                            self.app.get_eligible()
                        } else {
                            self.app.get_all_facts()
                        };

                        ui.label("Viewing Facts!");

                        ui.separator();

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            if !list.is_empty() {
                                list.into_iter().enumerate().for_each(
                                    |(index, f): (usize, Fact)| {
                                        ui.horizontal(|ui| {
                                            ui.label(format!("Term - {}, ", f.term));
                                            if *show_defs {
                                                ui.label(format!("Definition - {}", f.definition));
                                            } else {
                                                ui.label("Definition Hidden!");
                                            }

                                            if ui.button("Delete fact").clicked() {
                                                self.app.delete_at_index(index);
                                            }
                                        });
                                    },
                                );
                            } else {
                                ui.label("No facts");
                            }
                        });
                    }
                    JankiState::Csv {
                        file_name,
                        overwrite_existing,
                    } => {
                        ui.label("CSV Utilities");
                        ui.horizontal(|ui| {
                            ui.label("File Path");
                            ui.text_edit_singleline(file_name);
                        });
                        ui.checkbox(overwrite_existing, "Overwrite existing");

                        ui.separator();

                        if ui.button("Export current facts").clicked() {
                            event!(
                                Level::TRACE,
                                file_name,
                                overwrite_existing,
                                "Exporting current facts"
                            );
                            let mut facts_to_write = self.app.get_all_facts();
                            if !*overwrite_existing {
                                match File::open(&file_name) {
                                    Ok(file) => match read_in(file) {
                                        Err(e) => {
                                            error!("Error parsing CSV file: {e:?}");
                                            //TOOD: communicate this to user
                                        }
                                        Ok(csv_conts) => {
                                            facts_to_write.extend(csv_conts.into_iter());
                                        }
                                    },
                                    Err(e) => error!("Error reading csv file: {e}"),
                                }
                            }

                            match File::create(&file_name) {
                                Ok(file) => {
                                    facts_to_write.sort();
                                    write_out(file, facts_to_write)
                                        .unwrap_or_else(|err| error!("Error writing out: {err}"))
                                }
                                Err(e) => error!("Error creating file: {e}"),
                            }
                        }

                        if ui.button("Import new facts").clicked() {
                            {
                                let f = file_name.clone();
                                event!(Level::TRACE, f, overwrite_existing, "Importing new facts");
                            }
                            match File::open(file_name.clone()) {
                                Ok(file) => match read_in(file) {
                                    Err(e) => {
                                        error!("Error reading in CSV file: {e:?}");
                                        //TOOD: communicate this to user
                                    }
                                    Ok(csv_conts) => {
                                        if *overwrite_existing {
                                            self.app.clear();
                                        }
                                        self.app.add_facts(csv_conts);
                                    }
                                },
                                Err(e) => error!("Error reading in CSV file: {e}"),
                            }
                        }
                    }
                }
            });
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("Loading...");
                ui.spinner();
            });
        }
    }

    fn save(&mut self, mut storage: &mut dyn EStorage) {
        if !matches!(self.state, JankiState::Testing { .. }) {
            if self.has_done_initial_read {
                self.app
                    .write_custom(&mut storage as &mut dyn JStorage<ErrorType = serde_json::Error>)
                    .expect("Failure to write to EGUI storage");
            } else {
                self.has_done_initial_read = true;
                trace!("Doing initial read");
                self.app
                    .read_custom(&storage as &dyn JStorage<ErrorType = serde_json::Error>)
                    .expect("Failure to read from EGUI storage");
            }
        }
    }

    fn on_exit(&mut self, _gl: &eframe::glow::Context) {
        self.app.exit();

        if cfg!(feature = "opentel") {
            opentelemetry::global::shutdown_tracer_provider();
        }
    }

    fn auto_save_interval(&self) -> Duration {
        if !self.has_done_initial_read {
            Duration::from_millis(20)
        } else {
            Duration::from_secs(30) //the normal behaviourr
        }
    }
}
