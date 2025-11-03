use crate::hocon_parser::{HoconDocument, HoconValue};
use indexmap::IndexMap;

/// Represents the type of search match
#[derive(Debug, Clone, PartialEq)]
pub enum SearchMatch {
    Key,   // Matched in key name
    Value, // Matched in value
    Both,  // Matched in both
    None,  // No match
}

/// Check if a value matches the search query (recursive)
pub fn value_matches_search(value: &HoconValue, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    let query_lower = query.to_lowercase();

    match value {
        HoconValue::Int(n) => n.to_string().contains(&query_lower),
        HoconValue::Float(f) => f.to_string().contains(&query_lower),
        HoconValue::String(s) => s.to_lowercase().contains(&query_lower),
        HoconValue::Bool(b) => b.to_string().contains(&query_lower),
        HoconValue::Object(map) => {
            // Search recursively in objects
            map.iter().any(|(k, v)| {
                k.to_lowercase().contains(&query_lower) || value_matches_search(v, query)
            })
        }
        HoconValue::Array(arr) => {
            // Search in array items
            arr.iter().any(|v| value_matches_search(v, query))
        }
    }
}

/// Determine match type for highlighting
pub fn get_search_match(key: &str, value: &HoconValue, query: &str) -> SearchMatch {
    if query.is_empty() {
        return SearchMatch::None;
    }

    let key_matches = key.to_lowercase().contains(&query.to_lowercase());
    let value_matches = value_matches_search(value, query);

    match (key_matches, value_matches) {
        (true, true) => SearchMatch::Both,
        (true, false) => SearchMatch::Key,
        (false, true) => SearchMatch::Value,
        (false, false) => SearchMatch::None,
    }
}

/// Count search matches in document
pub fn count_search_matches(document: &HoconDocument, query: &str) -> (usize, usize, usize) {
    if query.is_empty() {
        return (0, 0, 0);
    }

    let mut total = 0;
    let mut key_matches = 0;
    let mut value_matches = 0;

    for (key, value) in document.root() {
        match get_search_match(key, value, query) {
            SearchMatch::Key => {
                total += 1;
                key_matches += 1;
            }
            SearchMatch::Value => {
                total += 1;
                value_matches += 1;
            }
            SearchMatch::Both => {
                total += 1;
                key_matches += 1;
                value_matches += 1;
            }
            SearchMatch::None => {}
        }
    }

    (total, key_matches, value_matches)
}

/// Renders a HOCON value with appropriate UI control
/// Returns true if the value was modified
pub fn render_hocon_value(
    ui: &mut egui::Ui,
    key: &str,
    value: &mut HoconValue,
    search_query: &str,
) -> bool {
    // Get search match type
    let search_match = get_search_match(key, value, search_query);

    // Filter by search query - skip if no match
    if !search_query.is_empty() && matches!(search_match, SearchMatch::None) {
        return false;
    }

    // Determine colors based on match type
    let key_color = match search_match {
        SearchMatch::Key | SearchMatch::Both => egui::Color32::YELLOW,
        _ => ui.style().visuals.text_color(),
    };

    let value_highlighted = matches!(search_match, SearchMatch::Value | SearchMatch::Both);

    match value {
        HoconValue::Int(n) => render_int_field(ui, key, n, key_color, value_highlighted),
        HoconValue::Float(f) => render_float_field(ui, key, f, key_color, value_highlighted),
        HoconValue::String(s) => render_string_field(ui, key, s, key_color, value_highlighted),
        HoconValue::Bool(b) => render_bool_field(ui, key, b, key_color, value_highlighted),
        HoconValue::Object(map) => render_object(ui, key, map, search_query),
        HoconValue::Array(arr) => render_array(ui, key, arr, search_query),
    }
}

fn render_int_field(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut i64,
    key_color: egui::Color32,
    value_highlighted: bool,
) -> bool {
    ui.horizontal(|ui| {
        ui.label("🔢");
        ui.colored_label(key_color, format!("{}:", label));

        if value_highlighted {
            let visuals = ui.visuals_mut();
            visuals.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(0, 200, 0, 50);
        }

        ui.add(egui::DragValue::new(value).speed(1.0))
    })
    .inner
    .changed()
}

fn render_float_field(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f64,
    key_color: egui::Color32,
    value_highlighted: bool,
) -> bool {
    ui.horizontal(|ui| {
        ui.label("🔢");
        ui.colored_label(key_color, format!("{}:", label));

        if value_highlighted {
            let visuals = ui.visuals_mut();
            visuals.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(0, 200, 0, 50);
        }

        ui.add(egui::DragValue::new(value).speed(0.1))
    })
    .inner
    .changed()
}

fn render_string_field(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    key_color: egui::Color32,
    value_highlighted: bool,
) -> bool {
    ui.horizontal(|ui| {
        ui.label("📝");
        ui.colored_label(key_color, format!("{}:", label));

        if value_highlighted {
            let visuals = ui.visuals_mut();
            visuals.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(0, 200, 0, 50);
        }

        ui.text_edit_singleline(value)
    })
    .inner
    .changed()
}

fn render_bool_field(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut bool,
    key_color: egui::Color32,
    value_highlighted: bool,
) -> bool {
    ui.horizontal(|ui| {
        ui.label("☑️");
        ui.colored_label(key_color, format!("{}:", label));

        if value_highlighted {
            ui.colored_label(egui::Color32::LIGHT_GREEN, "●");
        }

        ui.checkbox(value, "")
    })
    .inner
    .changed()
}

fn render_object(
    ui: &mut egui::Ui,
    label: &str,
    object: &mut IndexMap<String, HoconValue>,
    search_query: &str,
) -> bool {
    let mut changed = false;

    ui.collapsing(format!("📁 {}", label), |ui| {
        ui.indent(label, |ui| {
            for (key, value) in object.iter_mut() {
                changed |= render_hocon_value(ui, key, value, search_query);
            }
        });
    });

    changed
}

fn render_array(
    ui: &mut egui::Ui,
    label: &str,
    array: &mut Vec<HoconValue>,
    search_query: &str,
) -> bool {
    let mut changed = false;

    // Detect array type for better rendering
    let array_type = detect_array_type(array);

    match array_type {
        ArrayType::Empty => {
            ui.label(format!("📋 {}: []", label));
        }
        ArrayType::HomogeneousString => {
            changed |= render_string_array_inline(ui, label, array);
        }
        ArrayType::HomogeneousNumber => {
            changed |= render_number_array_inline(ui, label, array);
        }
        ArrayType::ObjectArray => {
            changed |= render_object_array_cards(ui, label, array, search_query);
        }
        ArrayType::Mixed => {
            changed |= render_array_mixed(ui, label, array, search_query);
        }
    }

    changed
}

#[derive(Debug, Clone, PartialEq)]
enum ArrayType {
    Empty,
    HomogeneousString,
    HomogeneousNumber,
    ObjectArray,
    Mixed,
}

fn detect_array_type(arr: &[HoconValue]) -> ArrayType {
    if arr.is_empty() {
        return ArrayType::Empty;
    }

    let all_strings = arr.iter().all(|v| matches!(v, HoconValue::String(_)));
    let all_numbers = arr
        .iter()
        .all(|v| matches!(v, HoconValue::Int(_) | HoconValue::Float(_)));
    let all_objects = arr.iter().all(|v| matches!(v, HoconValue::Object(_)));

    if all_strings {
        ArrayType::HomogeneousString
    } else if all_numbers {
        ArrayType::HomogeneousNumber
    } else if all_objects {
        ArrayType::ObjectArray
    } else {
        ArrayType::Mixed
    }
}

fn render_string_array_inline(ui: &mut egui::Ui, label: &str, array: &mut Vec<HoconValue>) -> bool {
    let mut changed = false;

    ui.collapsing(format!("📋 {}: {} strings", label, array.len()), |ui| {
        ui.indent(label, |ui| {
            // Show strings in a more compact format
            for (idx, item) in array.iter_mut().enumerate() {
                if let HoconValue::String(s) = item {
                    ui.horizontal(|ui| {
                        ui.label(format!("[{}]", idx));
                        if ui.text_edit_singleline(s).changed() {
                            changed = true;
                        }
                    });
                }
            }
        });
    });

    changed
}

fn render_number_array_inline(ui: &mut egui::Ui, label: &str, array: &mut Vec<HoconValue>) -> bool {
    let mut changed = false;

    ui.collapsing(format!("📋 {}: {} numbers", label, array.len()), |ui| {
        ui.indent(label, |ui| {
            // Show numbers in grid layout
            ui.horizontal_wrapped(|ui| {
                for (idx, item) in array.iter_mut().enumerate() {
                    match item {
                        HoconValue::Int(n) => {
                            ui.horizontal(|ui| {
                                ui.label(format!("[{}]:", idx));
                                if ui.add(egui::DragValue::new(n).speed(1.0)).changed() {
                                    changed = true;
                                }
                            });
                        }
                        HoconValue::Float(f) => {
                            ui.horizontal(|ui| {
                                ui.label(format!("[{}]:", idx));
                                if ui.add(egui::DragValue::new(f).speed(0.1)).changed() {
                                    changed = true;
                                }
                            });
                        }
                        _ => {}
                    }
                }
            });
        });
    });

    changed
}

fn render_object_array_cards(
    ui: &mut egui::Ui,
    label: &str,
    array: &mut Vec<HoconValue>,
    search_query: &str,
) -> bool {
    let mut changed = false;

    ui.collapsing(format!("📋 {}: {} entries", label, array.len()), |ui| {
        ui.indent(label, |ui| {
            for (idx, item) in array.iter_mut().enumerate() {
                if let HoconValue::Object(map) = item {
                    ui.group(|ui| {
                        ui.strong(format!("Entry {}", idx + 1));
                        ui.separator();

                        for (key, value) in map.iter_mut() {
                            changed |= render_hocon_value(ui, key, value, search_query);
                        }
                    });
                    ui.add_space(5.0);
                }
            }
        });
    });

    changed
}

fn render_array_mixed(
    ui: &mut egui::Ui,
    label: &str,
    array: &mut Vec<HoconValue>,
    search_query: &str,
) -> bool {
    let mut changed = false;

    ui.collapsing(
        format!("📋 {}: {} items (mixed)", label, array.len()),
        |ui| {
            ui.indent(label, |ui| {
                for (idx, item) in array.iter_mut().enumerate() {
                    let icon = match item {
                        HoconValue::Int(_) | HoconValue::Float(_) => "🔢",
                        HoconValue::String(_) => "📝",
                        HoconValue::Bool(_) => "☑️",
                        HoconValue::Object(_) => "📁",
                        HoconValue::Array(_) => "📋",
                    };

                    changed |=
                        render_hocon_value(ui, &format!("{} [{}]", icon, idx), item, search_query);
                }
            });
        },
    );

    changed
}

/// Category definition for organized display
pub struct Category {
    pub name: &'static str,
    pub icon: &'static str,
    pub fields: Vec<&'static str>,
}

impl Category {
    pub fn new(name: &'static str, icon: &'static str, fields: Vec<&'static str>) -> Self {
        Self { name, icon, fields }
    }
}

/// Get predefined categories for the savegame
pub fn get_categories() -> Vec<Category> {
    vec![
        Category::new(
            "Player Stats",
            "⭐",
            vec![
                "gems",
                "gold",
                "gemsTotal",
                "highestLevel",
                "lastCharacter",
                "gameMode",
                "personalBestGold",
                "welcomeGems",
            ],
        ),
        Category::new("Achievements", "🏆", vec!["achievements"]),
        Category::new("Challenges", "🎯", vec!["challenges", "completeChallenges"]),
        Category::new(
            "Collectables",
            "📦",
            vec!["collectables", "collectables_new"],
        ),
        Category::new("Highest Levels", "🎮", vec!["highestLevels"]),
        Category::new("Statistics", "📊", vec!["dailyStats", "totalStats"]),
        Category::new("High Scores", "🏅", vec!["highscores"]),
        Category::new(
            "Leaderboards",
            "🌐",
            vec![
                "leaderboard_bombs_exploded",
                "leaderboard_distance_ran",
                "leaderboard_enemies_killed",
                "leaderboard_most_gold",
            ],
        ),
        Category::new("McDoodle", "🎨", vec!["mcdoodle"]),
        Category::new("Perms", "🔑", vec!["perms"]),
        Category::new(
            "Metadata",
            "ℹ",
            vec![
                "challengeDate",
                "statsDay",
                "version",
                "statsVersion",
                "permsVersion",
                "replayCode",
            ],
        ),
    ]
}

/// Render a category with its fields
pub fn render_category(
    ui: &mut egui::Ui,
    category: &Category,
    document: &mut HoconDocument,
    search_query: &str,
) -> bool {
    let mut changed = false;

    ui.collapsing(format!("{} {}", category.icon, category.name), |ui| {
        ui.indent(category.name, |ui| {
            for field_name in &category.fields {
                if let Some(value) = document.root_mut().get_mut(*field_name) {
                    changed |= render_hocon_value(ui, field_name, value, search_query);
                }
            }
        });
    });

    changed
}

/// Render all uncategorized fields
pub fn render_uncategorized(
    ui: &mut egui::Ui,
    document: &mut HoconDocument,
    categories: &[Category],
    search_query: &str,
) -> bool {
    // Collect all categorized field names
    let mut categorized_fields = std::collections::HashSet::new();
    for category in categories {
        for field in &category.fields {
            categorized_fields.insert(*field);
        }
    }

    let mut changed = false;
    let mut has_uncategorized = false;

    // Find uncategorized fields
    let uncategorized: Vec<String> = document
        .root()
        .keys()
        .filter(|k| !categorized_fields.contains(k.as_str()))
        .cloned()
        .collect();

    if !uncategorized.is_empty() {
        has_uncategorized = true;
    }

    if has_uncategorized {
        ui.collapsing("🔧 Other Fields", |ui| {
            ui.indent("other", |ui| {
                for key in &uncategorized {
                    if let Some(value) = document.root_mut().get_mut(key) {
                        changed |= render_hocon_value(ui, key, value, search_query);
                    }
                }
            });
        });
    }

    changed
}
