use serenity::framework::StandardFramework;
use serenity::http::Http;
use std::collections::HashSet;
use std::env;

pub(crate) async fn build_framework(discord_token: &str) -> StandardFramework {
    let http = Http::new(discord_token);

    let owners = match http.get_current_application_info().await {
        Ok(info) => HashSet::from([info.owner.id]),
        Err(err) => panic!("Could not access application info: {:?}", err),
    };

    StandardFramework::new().configure(|c| {
        c.owners(owners)
            .prefix(&env::var("DISCORD_CMD_PREFIX").unwrap_or_else(|_| "!".to_string()))
    })
    // .group()
}
