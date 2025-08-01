use std::collections::HashMap;

use analysis_template::{base::cheat_analyser_base::CheatAnalyser, Detection};
use eframe::egui;
use itertools::Itertools;
use tf_demo_parser::Demo;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 800.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "Demo Analysis",
        options,
        Box::new(|_cc| Ok(Box::new(Gui::new()))),
    )
}

#[derive(Default)]
struct Gui {
    algos: HashMap<String, bool>,
    params: HashMap<String, HashMap<&'static str, f32>>,
    file: Option<std::path::PathBuf>,
    processing: bool,
    detections: HashMap<u64, Vec<Detection>>,
    selected_player: Option<u64>,
    selected_detection: Option<usize>,

    analyser: Option<CheatAnalyser<'static>>,
}

impl Gui {
    pub fn new() -> Self {
        let mut params = HashMap::new();
        for mut a in analysis_template::algorithms().drain(..) {
            if a.params().is_some() {
                params.insert(a.algorithm_name().to_string(), a.params().cloned().unwrap());
            }
        }
        if let Ok(data) = std::fs::read_to_string("params.json") {
            if let Ok(saved_params) =
                serde_json::from_str::<HashMap<String, HashMap<String, f32>>>(&data)
            {
                for saved_algo in saved_params {
                    if let Some(algo) = params.get_mut(&saved_algo.0) {
                        for saved_param in saved_algo.1 {
                            if let Some(param) = algo.get_mut(saved_param.0.as_str()) {
                                *param = saved_param.1;
                            }
                        }
                    }
                }
            }
        }
        Self {
            algos: HashMap::from_iter(
                analysis_template::algorithms()
                    .iter()
                    .map(|a| (a.algorithm_name().to_string(), a.default())),
            ),
            params,
            ..Default::default()
        }
    }

    fn analyse(&mut self) {
        if self.file.is_none() {
            return;
        }
        self.selected_detection = None;
        self.selected_player = None;
        let mut algorithms = analysis_template::algorithms();
        algorithms.retain(|a| self.algos[a.algorithm_name()]);

        for a in algorithms.iter_mut() {
            if let Some(p) = self.params.get(a.algorithm_name()) {
                a.params().as_mut().unwrap().clone_from(p);
            }
        }

        let file = std::fs::read(self.file.as_ref().unwrap()).unwrap();
        let demo: Demo = Demo::new(&file);
        let analyser = analysis_template::analyse(&demo, algorithms).unwrap();
        self.analyser = Some(analyser);
        self.detections.clear();
        for det in self.analyser.as_ref().unwrap().detections.clone() {
            self.detections.entry(det.player).or_default().push(det);
        }
        self.analyser.as_ref().unwrap().print_detection_summary();
    }
}

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let hovered = !ctx.input(|i| i.raw.hovered_files.is_empty());
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.processing {
                ui.disable();
            }
            ui.horizontal(|ui|{
                ui.vertical(|ui|{
                    ui.heading("Algorithms");
                    for mut algo in self.algos.iter_mut().sorted_by_key(|a| a.0) {
                        ui.checkbox(&mut algo.1, algo.0);
                    }
                });
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);
                ui.vertical(|ui|{
                    ui.horizontal(|ui|{
                        ui.heading("Parameters");
                        if ui.button("Save").clicked(){
                            std::fs::write("params.json", &serde_json::to_vec_pretty(&self.params).unwrap()).unwrap();
                        }
                    });
                    ui.separator();
                    for (name, params) in self.params.iter_mut().sorted_by_key(|a|a.0) {
                        if !self.algos[name]{
                            continue;
                        }
                        ui.add_space(10.0);
                        ui.heading(name);
                        ui.separator();
                        for param in params.iter_mut().sorted_by_key(|p|p.0){
                            ui.horizontal(|ui|{
                                ui.add(egui::DragValue::new(param.1).max_decimals(50));
                                ui.label(*param.0);
                            });
                        }
                    }
                });
            });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Open...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Demos", &["dem"])
                        .pick_file()
                    {
                        self.file = Some(path);
                        self.analyse();
                    }
                }
                if self.file.is_some() {
                    if ui.button("Analyse").clicked() {
                        self.analyse();
                    }
                    if ui.button("Save detections").clicked() {
                        if let Some(a) = &self.analyser {
                            if let Some(path) = rfd::FileDialog::new().set_file_name("detections.json").save_file(){
                                let analysis = serde_json::json!({
                                    "server_ip": a.header.as_ref().map_or("unknown".to_string(), |h| h.server.clone()),
                                    "duration": a.tick,
                                    "author": a.header.as_ref().map_or("unknown".to_string(), |h| h.nick.clone()),
                                    "map": a.header.as_ref().map_or("unknown".to_string(), |h| h.map.clone()),
                                    "detections": a.detections
                                });
                                std::fs::write(path, serde_json::to_vec_pretty(&analysis).unwrap()).unwrap();
                            }
                        }
                    }
                }
                if hovered {
                    ui.label("Drop to analyse");
                }
            });
            ui.add_space(10.0);
            if let Some(p) = &self.file {
                ui.heading(p.file_name().unwrap().to_string_lossy());
                ui.label("Doubleclick steamid to open profile");
            }
            ui.separator();
            ui.horizontal_top(|ui| {
                egui::ScrollArea::vertical()
                    .id_salt("players")
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            for player in self.detections.iter().sorted_by_key(|d| d.1.len()).rev()
                            {
                                let res = ui.selectable_label(
                                    self.selected_player.is_some_and(|u| u == *player.0),
                                    format!("{} ({})", player.0, player.1.len()),
                                );
                                if res.clicked() {
                                    self.selected_player = Some(*player.0);
                                    self.selected_detection = None;
                                }
                                if res.double_clicked() {
                                    let _ = opener::open_browser(format!(
                                        "https://steamcommunity.com/profiles/{}",
                                        player.0
                                    ));
                                }
                            }
                        });
                    });
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("detections")
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            if let Some(detections) =
                                self.selected_player.and_then(|p| self.detections.get(&p))
                            {
                                for (i, det) in detections.iter().enumerate() {
                                    if ui
                                        .selectable_label(
                                            self.selected_detection.is_some_and(|si| si == i),
                                            format!("{}: {}", det.tick, det.algorithm),
                                        )
                                        .clicked()
                                    {
                                        self.selected_detection = Some(i);
                                    }
                                }
                            }
                        });
                    });
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("details")
                    .show(ui, |ui| {
                        if let Some(det) = self
                            .selected_player
                            .and_then(|p| self.detections.get(&p))
                            .and_then(|dets| self.selected_detection.and_then(|di| dets.get(di)))
                        {
                            ui.label(serde_json::to_string_pretty(&det.data).unwrap());
                        }
                    });
            });
        });
        if let Some(f) = ctx.input(|i| i.raw.dropped_files.first().cloned()) {
            self.file = f.path;
            self.analyse();
        }
    }
}
