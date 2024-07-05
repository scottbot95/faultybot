use poise::serenity_prelude as serenity;
use serenity::Mentionable;
use crate::{Context, Error};
use crate::permissions::{Permission, validate_access};

#[derive(poise::Modal)]
#[name = "Feedback"]
struct FeedbackModal {
    title: String,
    #[paragraph]
    description: String,
}

/// Submit a feedback to FaultyBot
#[poise::command(
    slash_command,
    user_cooldown = 15,
)]
pub async fn feedback(
    ctx: Context<'_>,
    #[rename = "type"] label: FeedbackTypeChoice
) -> Result<(), Error> {
    use poise::Modal as _;
    use std::fmt::Write as _;

    validate_access(&ctx, Permission::SendFeedback(Some(label.label()))).await?;

    let ctx = match ctx {
        Context::Application(ctx) => ctx,
        _ => unreachable!()
    };

    let data = ctx.data();
    let gh_config = data.config.github.as_ref().unwrap();
    let octocrab = data.octocrab.as_ref().unwrap();

    let confirmation_channel = gh_config.confirmation_channel;

    let data = FeedbackModal::execute(ctx).await?;

    if let Some(data) = data {
        let issue_title = format!("[{}] {}", label, data.title);
        let mut issue_description = format!(
            "{}\n\n---\n\nRequested by: {} ({})",
            data.description, ctx.author().id.mention(), ctx.author().name
        );
        if let Some(guild) = ctx.guild_id() {
            let guild_name =guild.name(ctx.cache()).unwrap_or_else(|| "Unknown".to_string());
            write!(&mut issue_description, " in guild {} ({})", guild, guild_name)?;
        }

        ctx.send(poise::CreateReply::default()
            .content("Submitted suggestion")
            .embed(serenity::CreateEmbed::default()
                .title(&data.title)
                .description(&issue_description))
            .ephemeral(true))
            .await?;

        let confirmation_message = confirmation_channel.send_message(
            ctx.http(),
            serenity::CreateMessage::default()
                .embed(serenity::CreateEmbed::default()
                    .title(&data.title)
                    .description(&issue_description))
                .components(vec![
                    serenity::CreateActionRow::Buttons(vec![
                        serenity::CreateButton::new("accept")
                            .label("Accept")
                            .style(serenity::ButtonStyle::Success)
                            .emoji('✅'),
                        serenity::CreateButton::new("decline")
                            .label("Decline")
                            .style(serenity::ButtonStyle::Danger)
                            .emoji('❌'),
                    ])
                ])
        ).await?;

        let interaction = confirmation_message.await_component_interaction(ctx.serenity_context().shard.clone()).await;
        if let Some(interaction) = interaction {
            if interaction.data.custom_id == "accept" {
                tracing::info!("Submitting new GitHub issue: {}", issue_title);

                let issue = octocrab.issues(&gh_config.owner, &gh_config.repo)
                    .create(&issue_title)
                    .body(&issue_description)
                    .labels(vec![label.label()])
                    .send()
                    .await?;

                let issue_url = issue.html_url;

                confirmation_channel.send_message(
                    ctx.http(),
                    serenity::CreateMessage::default()
                        .content(format!("Issue submitted: {}", issue_url))
                ).await?;

                ctx.send(poise::CreateReply::default()
                    .content(format!("Your suggestion has been accepted. {}", issue_url))
                    .ephemeral(true))
                    .await?;
            } else if interaction.data.custom_id == "decline" {
                // Don't care, just delete the interaction
            } else {
                tracing::warn!("Unexpected interaction response: {:?}", interaction);
            }
            tracing::debug!("{:?}", interaction.data);
            confirmation_message.delete(ctx.http(), None).await?;
        } else {
            tracing::warn!("Returned interaction was None for: {:?}", confirmation_message);
        }
    }

    Ok(())
}

#[derive(Debug, Copy, Clone, poise::ChoiceParameter, derive_more::Display)]
pub enum FeedbackTypeChoice {
    Bug,
    Suggestion,
}

impl FeedbackTypeChoice {
    fn label(self) -> String {
        match self {
            FeedbackTypeChoice::Bug => "bug",
            FeedbackTypeChoice::Suggestion => "enhancement",
        }.to_string()
    }
}