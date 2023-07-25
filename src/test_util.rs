static INIT: std::sync::Once = std::sync::Once::new();

pub fn setup_env() {
    INIT.call_once(|| {
        dotenvy::dotenv().unwrap();
    })
}

pub async fn test_database() -> crate::database::Database {
    let settings: crate::settings::config::FaultybotConfig =
        crate::settings::config::build_config(None)
            .unwrap()
            .try_deserialize()
            .unwrap();
    let db = crate::database::Database::connect(&settings.database)
        .await
        .expect("Failed to connect to db");

    db
}
