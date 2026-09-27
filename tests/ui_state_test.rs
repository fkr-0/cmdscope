use cmdscope::{AppConfig, ColumnConfig, ColumnId, SortDirection, SortField, UiState};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_state_path(name: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir()
        .join(format!("cmdscope-{name}-{}-{nonce}", std::process::id()))
        .join("ui-state.toml")
}

#[test]
fn ui_state_round_trips_atomically() {
    let path = temp_state_path("roundtrip");
    let columns = ColumnConfig {
        date: true,
        pwd: false,
        exit: true,
        duration: false,
        order: vec![
            ColumnId::Exit,
            ColumnId::Date,
            ColumnId::Pwd,
            ColumnId::Duration,
        ],
        sort_by: SortField::Exit,
        sort_direction: SortDirection::Ascending,
        ..ColumnConfig::default()
    };
    let state = UiState::from_columns(&columns);

    state.save_atomic(&path).unwrap();
    assert_eq!(UiState::load_optional(&path).unwrap(), Some(state));

    let directory = path.parent().unwrap();
    assert_eq!(
        fs::read_dir(directory)
            .unwrap()
            .filter_map(Result::ok)
            .count(),
        1
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn config_file_overrides_persisted_state_field_by_field() {
    let persisted = UiState {
        version: 1,
        order: vec![
            ColumnId::Pwd,
            ColumnId::Date,
            ColumnId::Exit,
            ColumnId::Duration,
        ],
        visible: vec![ColumnId::Pwd, ColumnId::Date],
        sort_by: SortField::Date,
        sort_direction: SortDirection::Ascending,
    };

    let config = AppConfig::from_toml_layered(
        r#"
        [ui.columns]
        pwd = false
        exit = true
        sort_direction = "descending"
        "#,
        Some(&persisted),
    )
    .unwrap();

    assert_eq!(
        config.ui.columns.normalized_order(),
        vec![
            ColumnId::Pwd,
            ColumnId::Date,
            ColumnId::Exit,
            ColumnId::Duration,
        ]
    );
    assert_eq!(
        config.ui.columns.visible(),
        vec![ColumnId::Date, ColumnId::Exit]
    );
    assert_eq!(config.ui.columns.sort_by, SortField::Date);
    assert_eq!(config.ui.columns.sort_direction, SortDirection::Descending);
}

#[test]
fn persisted_state_rejects_duplicate_column_order() {
    let state = UiState {
        version: 1,
        order: vec![ColumnId::Date, ColumnId::Date],
        visible: vec![ColumnId::Date],
        sort_by: SortField::Relevance,
        sort_direction: SortDirection::Descending,
    };
    let mut columns = ColumnConfig::default();

    let error = state.apply_to(&mut columns).unwrap_err().to_string();
    assert!(error.contains("appears more than once"));
}
