use poise::{CreateReply, serenity_prelude as serenity};
use poise::serenity_prelude::{all, ComponentInteraction};
use serenity::Builder as _;
use crate::{Context, Data, Error, Result};
use crate::error::InternalError;
use crate::util::say_ephemeral;
use crate::wizard::{Wizard, WizardStep};

/// Manage permissions for a given principle
#[poise::command(
    slash_command,
    guild_only,
    subcommands("create"),
    rename = "permissions_v2"
)]
pub async fn permissions(_ctx: Context<'_>) -> Result<()> {
    Ok(())
}

#[derive(Debug, poise::Modal)]
#[name = "Specifier"]
struct SpecifierModal {
    specifier: String,
}

#[derive(Debug, Default)]
struct PrincipleStep {
    roles: Vec<serenity::RoleId>,
    channels: Vec<serenity::ChannelId>,
    users: Vec<serenity::UserId>,
}

#[poise::async_trait]
impl WizardStep<crate::Data, crate::Error>  for PrincipleStep {
    type State = PermissionWizard;

    fn create_reply(&self, _input: &Self::State) -> CreateReply {
        poise::CreateReply::new()
            .content(build_prompt(PROMPT_PREFIX, &self.channels, &self.roles, &self.users).unwrap())
            .ephemeral(true)
            .components(vec![
                serenity::CreateActionRow::SelectMenu(
                    serenity::CreateSelectMenu::new("role", serenity::CreateSelectMenuKind::Role)
                        .placeholder("Select roles:")
                        .min_values(0)
                        .max_values(25) // discord max is 25
                ),
                serenity::CreateActionRow::SelectMenu(
                    serenity::CreateSelectMenu::new("channel", serenity::CreateSelectMenuKind::Channel {
                        channel_types: None,
                    })
                        .placeholder("Select Channels:")
                        .min_values(0)
                        .max_values(25) // discord max is 25
                ),
                serenity::CreateActionRow::SelectMenu(
                    serenity::CreateSelectMenu::new("user", serenity::CreateSelectMenuKind::User)
                        .placeholder("Select users:")
                        .min_values(0)
                        .max_values(25) // discord max is 25
                ),
            ])
    }

    async fn handle_interaction(&mut self, _ctx: crate::Context<'_>, interaction: serenity::ComponentInteraction) -> Result<bool> {
        match interaction.data.kind {
            serenity::ComponentInteractionDataKind::ChannelSelect { values } => self.channels = values,
            serenity::ComponentInteractionDataKind::RoleSelect { values } => self.roles = values,
            serenity::ComponentInteractionDataKind::UserSelect { values } => self.users = values,
            _ => return Ok(false)
        }

        Ok(true)
    }

}

#[derive(Debug, Default)]
struct ResourceStep {
    resource: Option<String>,
    specifier: Option<String>,
}

#[poise::async_trait]
impl WizardStep<crate::Data, crate::Error> for ResourceStep {
    type State = PermissionWizard;

    fn is_valid(&self) -> bool {
        self.resource.is_some()
    }

    fn create_reply(&self, state: &PermissionWizard) -> CreateReply {
        let principle = state.principle.as_ref().unwrap();
        let prompt = build_prompt("What resource to manage permissions for?", &principle.channels, &principle.roles, &principle.users).unwrap();
        poise::CreateReply::default()
            .content(prompt)
            .components(vec![
                serenity::CreateActionRow::SelectMenu(
                    serenity::CreateSelectMenu::new("resource", serenity::CreateSelectMenuKind::String {
                        options: vec![
                            serenity::CreateSelectMenuOption::new("Persona", "persona")
                                .description("Manage or chat with a persona")
                                .default_selection(self.resource == Some("persona".to_string())),
                            serenity::CreateSelectMenuOption::new("Permissions", "permissions")
                                .description("Manage permissions")
                                .default_selection(self.resource == Some("permissions".to_string())),
                            serenity::CreateSelectMenuOption::new("Settings", "settings")
                                .description("Manage settings")
                                .default_selection(self.resource == Some("settings".to_string())),
                            serenity::CreateSelectMenuOption::new("Feedback", "feedback")
                                .description("Ability to send feedback to FaultBot developers")
                                .default_selection(self.resource == Some("feedback".to_string())),
                        ]
                    }).placeholder("Select resource")
                )
            ])
    }

    async fn handle_interaction(&mut self, ctx: crate::Context<'_>, interaction: serenity::ComponentInteraction) -> Result<bool> {
        let slash_ctx = match ctx {
            Context::Application(ctx) => ctx,
            _ => unreachable!()
        };

        match interaction.data.kind {
            serenity::ComponentInteractionDataKind::StringSelect { values } => self.resource = values.first().cloned(),
            serenity::ComponentInteractionDataKind::Button => if interaction.data.custom_id.as_str() == "specifier" {
                use poise::Modal as _;
                let data = SpecifierModal::execute(slash_ctx).await?;
                self.specifier = data.map(|v| v.specifier);
            }
            _ => return Ok(false)
        }

        Ok(true)
    }
}

#[derive(Debug, Default)]
struct ActionsStep {
    selected_actions: Vec<String>
}

#[poise::async_trait]
impl WizardStep<crate::Data, crate::Error> for ActionsStep {
    type State = PermissionWizard;

    fn is_valid(&self) -> bool {
        !self.selected_actions.is_empty()
    }

    fn create_reply(&self, state: &Self::State) -> CreateReply {
        let principle = state.principle.as_ref().unwrap();
        let resource = state.resource.as_ref().and_then(|r| r.resource.as_ref()).unwrap(); // Next button doesn't work unless resource is set
        let options = match resource.as_str() {
            "persona" => vec![
                serenity::CreateSelectMenuOption::new("Create", "create")
                    .description("Create a new persona")
                    .default_selection(self.selected_actions.contains(&"create".to_string())),
                serenity::CreateSelectMenuOption::new("Chat", "chat")
                    .description("Chat with a persona")
                    .default_selection(self.selected_actions.contains(&"chat".to_string())),
            ],
            "permissions" => vec![
                serenity::CreateSelectMenuOption::new("Manage", "manage")
                    .description("Change permissions")
                    .default_selection(self.selected_actions.contains(&"manage".to_string())),
            ],
            "settings" => vec![
                serenity::CreateSelectMenuOption::new("Manage", "manage")
                    .description("Change settings")
                    .default_selection(self.selected_actions.contains(&"manage".to_string())),
            ],
            "feedback" => vec![serenity::CreateSelectMenuOption::new("Send", "send")
                .description("Send feedback")
                .default_selection(self.selected_actions.contains(&"send".to_string()))
            ],
            _ => unreachable!()
        };
        let num_actions = options.len() as u8;

        poise::CreateReply::default()
            .content(build_prompt(
                format!("What actions to manage for resource: {}", resource),
                &principle.channels,
                &principle.roles,
                &principle.users
            ).unwrap())
            .components(vec![
                serenity::CreateActionRow::SelectMenu(
                    serenity::CreateSelectMenu::new("actions", serenity::CreateSelectMenuKind::String {
                        options,
                    })
                        .placeholder("Select actions")
                        .min_values(1)
                        .max_values(num_actions)
                ),
            ])
    }

    async fn handle_interaction(&mut self, _ctx: poise::Context<'_, Data, Error>, interaction: serenity::ComponentInteraction) -> std::result::Result<bool, Error> {
        match interaction.data.kind {
            serenity::ComponentInteractionDataKind::StringSelect { values } => self.selected_actions = values,
            _ => return Ok(false)
        }

        Ok(true)
    }
}

#[derive(Debug, Default)]
struct PermissionWizard {
    principle: Option<PrincipleStep>,
    resource: Option<ResourceStep>,
    actions: Option<ActionsStep>,
}

#[poise::async_trait]
impl Wizard<Data, Error> for PermissionWizard {

    async fn execute<'a>(&mut self, ctx: crate::Context<'a>) -> Result<Option<poise::ReplyHandle<'a>>> {
        tracing::debug!("Running principle step");
        let mut principle = PrincipleStep::default();
        let mut msg = if let Some(msg) = crate::wizard::execute_step(
            &mut principle,
            ctx,
            self,
            None
        ).await? {
            msg
        } else {
            return Ok(None);
        };
        self.principle = Some(principle);

        tracing::debug!("Running resource step");
        let mut resource = ResourceStep::default();
        msg = if let Some(msg) = crate::wizard::execute_step(
            &mut resource,
            ctx,
            self,
            Some(msg)
        ).await? {
            msg
        } else {
            return Ok(None);
        };
        self.resource = Some(resource);

        tracing::debug!("Running actions step");
        let mut actions = ActionsStep::default();
        msg = if let Some(msg) = crate::wizard::execute_step(
            &mut actions,
            ctx,
            self,
            Some(msg)
        ).await? {
            msg
        } else {
            return Ok(None);
        };
        self.actions = Some(actions);

        tracing::debug!("Done!");

        Ok(Some(msg))
    }
}

#[poise::command(slash_command)]
pub async fn create(ctx: Context<'_>) -> Result<()> {
    let mut wizard = PermissionWizard::default();
    let handle = if let Some(msg) = wizard.execute(ctx).await? {
        msg
    } else {
        return Ok(());
    };

    let principle = wizard.principle.unwrap();
    let resource = wizard.resource.unwrap();
    let actions = wizard.actions.unwrap();
    let prefix = format!(
        "Changing permissions for\nResource: {}/{:?}\nActions:{:?}",
        resource.resource.unwrap(), resource.specifier, actions.selected_actions
    );
    let prompt = build_prompt(prefix, &principle.channels, &principle.roles, &principle.users)?;

    handle.edit(
        ctx,
        poise::CreateReply::default()
            .content(prompt)
            .components(vec![])
    ).await?;

    Ok(())
}

/// Manage permissions for a given principle
#[poise::command(slash_command)]
pub async fn create_old(ctx: Context<'_>) -> Result<()> {
    let slash_ctx = match ctx {
        Context::Application(ctx) => ctx,
        _ => unreachable!()
    };

    let handle = ctx.send(poise::CreateReply::new()
        .content(build_prompt(PROMPT_PREFIX, &[], &[], &[])?)
        .ephemeral(true)
        .components(vec![
            serenity::CreateActionRow::SelectMenu(
                serenity::CreateSelectMenu::new("role", serenity::CreateSelectMenuKind::Role)
                    .placeholder("Select roles:")
                    .min_values(0)
                    .max_values(25) // discord max is 25
            ),
            serenity::CreateActionRow::SelectMenu(
                serenity::CreateSelectMenu::new("channel", serenity::CreateSelectMenuKind::Channel {
                    channel_types: None,
                })
                    .placeholder("Select Channels:")
                    .min_values(0)
                    .max_values(25) // discord max is 25
            ),
            serenity::CreateActionRow::SelectMenu(
                serenity::CreateSelectMenu::new("user", serenity::CreateSelectMenuKind::User)
                    .placeholder("Select users:")
                    .min_values(0)
                    .max_values(25) // discord max is 25
            ),
            serenity::CreateActionRow::Buttons(vec![
                serenity::CreateButton::new("next")
                    .label("Next")
                    .style(serenity::ButtonStyle::Primary)
            ]),
        ])
    ).await?;
    let message = handle.message().await?;

    let mut roles = vec![];
    let mut channels = vec![];
    let mut users = vec![];

    let mut complete = false;

    while let Some(interaction) = message
        .await_component_interactions(ctx)
        .author_id(ctx.author().id)
        .timeout(std::time::Duration::from_secs(120))
        .await
    {
        match interaction.data.kind {
            serenity::ComponentInteractionDataKind::ChannelSelect { values } => channels = values,
            serenity::ComponentInteractionDataKind::RoleSelect { values } => roles = values,
            serenity::ComponentInteractionDataKind::UserSelect { values } => users = values,
            // stop listening on the next button
            serenity::ComponentInteractionDataKind::Button => if interaction.data.custom_id == "next" {
                complete = true;
                break;
            },
            kind => {
                tracing::error!("Unexpected data from interaction {:?}", kind);
                return Err(InternalError::UnexpectedInteraction("Unexpected data from interaction".to_string()).into());
            }
        }

        // Can't use interaction.create_response due to lifetime issues
        serenity::CreateInteractionResponse::Acknowledge
            .execute(ctx.serenity_context(), (interaction.id, &interaction.token))
            .await?;

        handle.edit(
            ctx,
            poise::CreateReply::default()
                .content(build_prompt(PROMPT_PREFIX, &channels, &roles, &users)?),
        ).await?;
    }

    if !complete {
        return Err(InternalError::Timeout("waiting for principle selection".to_string()).into());
    }

    handle.edit(
        ctx,
        poise::CreateReply::default()
            .content(build_prompt("What resource to manage permissions for?", &channels, &roles, &users)?)
            .components(vec![
                serenity::CreateActionRow::SelectMenu(
                    serenity::CreateSelectMenu::new("resource", serenity::CreateSelectMenuKind::String {
                        options: vec![
                            serenity::CreateSelectMenuOption::new("Persona", "persona")
                                .description("Manage or chat with a persona"),
                            serenity::CreateSelectMenuOption::new("Permissions", "permissions")
                                .description("Manage permissions"),
                            serenity::CreateSelectMenuOption::new("Settings", "settings")
                                .description("Manage settings"),
                            serenity::CreateSelectMenuOption::new("Feedback", "feedback")
                                .description("Ability to send feedback to FaultBot developers"),
                        ]
                    }).placeholder("Select resource")
                ),
                serenity::CreateActionRow::Buttons(vec![
                    // serenity::CreateButton::new("specifier")
                    //     .label("Edit Specifier"),
                    serenity::CreateButton::new("next")
                        .label("Next")
                        .style(serenity::ButtonStyle::Primary),
                ]),
            ]),
    ).await?;

    let mut resource: Option<String> = None;
    let mut specifier: Option<String> = None;
    let mut complete = false;
    while let Some(interaction) = message
        .await_component_interactions(ctx)
        .author_id(ctx.author().id)
        .timeout(std::time::Duration::from_secs(120))
        .await
    {
        match interaction.data.kind {
            serenity::ComponentInteractionDataKind::StringSelect { values } => resource = values.first().cloned(),
            // stop listening on the next button
            serenity::ComponentInteractionDataKind::Button => match interaction.data.custom_id.as_str() {
                "next" => {
                    if resource.is_some() {
                        complete = true;
                        break;
                    }
                    say_ephemeral(ctx, "Must specify a resource", true).await?;
                }
                "specifier" => {
                    use poise::Modal as _;
                    let data = SpecifierModal::execute(slash_ctx).await?;
                    specifier = data.map(|v| v.specifier);
                }
                _ => (),
            }
            kind => {
                tracing::error!("Unexpected data from interaction {:?}", kind);
                return Err(InternalError::UnexpectedInteraction("Unexpected data from interaction".to_string()).into());
            }
        }

        // Can't use interaction.create_response due to lifetime issues
        serenity::CreateInteractionResponse::Acknowledge
            .execute(ctx.serenity_context(), (interaction.id, &interaction.token))
            .await?;

        let mut prefix = "What resource to manage permissions for?".to_string();
        if let Some(specifier) = &specifier {
            prefix += format!("\nSpecifier: {}", specifier).as_str();
        }
        handle.edit(
            ctx,
            poise::CreateReply::default()
                .content(build_prompt(prefix, &channels, &roles, &users)?)
                .components(vec![
                    serenity::CreateActionRow::SelectMenu(
                        serenity::CreateSelectMenu::new("resource", serenity::CreateSelectMenuKind::String {
                            options: vec![
                                serenity::CreateSelectMenuOption::new("Persona", "persona")
                                    .description("Manage or chat with a persona")
                                    .default_selection(resource == Some("persona".to_string())),
                                serenity::CreateSelectMenuOption::new("Permissions", "permissions")
                                    .description("Manage permissions")
                                    .default_selection(resource == Some("permissions".to_string())),
                                serenity::CreateSelectMenuOption::new("Settings", "settings")
                                    .description("Manage settings")
                                    .default_selection(resource == Some("settings".to_string())),
                                serenity::CreateSelectMenuOption::new("Feedback", "feedback")
                                    .description("Ability to send feedback to FaultBot developers")
                                    .default_selection(resource == Some("feedback".to_string())),
                            ]
                        }).placeholder("Select resource")
                    ),
                    serenity::CreateActionRow::Buttons(vec![
                        // serenity::CreateButton::new("specifier")
                        //     .label("Edit Specifier"),
                        serenity::CreateButton::new("next")
                            .label("Next")
                            .style(serenity::ButtonStyle::Primary),
                    ]),
                ]),
        ).await?;
    }

    if !complete {
        return Err(InternalError::Timeout("waiting for resource selection".to_string()).into());
    }

    let resource = resource.unwrap(); // Next button doesn't work unless resource is set
    let actions = match resource.as_str() {
        "persona" => vec![
            serenity::CreateSelectMenuOption::new("Create", "create")
                .description("Create a new persona"),
            serenity::CreateSelectMenuOption::new("Chat", "chat")
                .description("Chat with a persona"),
        ],
        "permissions" => vec![
            serenity::CreateSelectMenuOption::new("Manage", "manage")
                .description("Change permissions"),
        ],
        "settings" => vec![
            serenity::CreateSelectMenuOption::new("Manage", "manage")
                .description("Change settings"),
        ],
        "feedback" => vec![serenity::CreateSelectMenuOption::new("Send", "send")
            .description("Send feedback")
        ],
        _ => unreachable!()
    };
    let num_actions = actions.len() as u8;

    handle.edit(
        ctx,
        poise::CreateReply::default()
            .content(build_prompt(format!("What actions to manage for resource: {}", resource), &channels, &roles, &users)?)
            .components(vec![
                serenity::CreateActionRow::SelectMenu(
                    serenity::CreateSelectMenu::new("actions", serenity::CreateSelectMenuKind::String {
                        options: actions.clone(),
                    })
                        .placeholder("Select actions")
                        .min_values(1)
                        .max_values(num_actions)
                ),
                serenity::CreateActionRow::Buttons(vec![
                    serenity::CreateButton::new("next")
                        .label("Next")
                        .style(serenity::ButtonStyle::Primary)
                        .disabled(true)
                ]),
            ]),
    ).await?;

    let mut selected_actions = vec![];
    let mut complete = false;
    while let Some(interaction) = message
        .await_component_interactions(ctx)
        .author_id(ctx.author().id)
        .timeout(std::time::Duration::from_secs(120))
        .await
    {
        match interaction.data.kind {
            serenity::ComponentInteractionDataKind::StringSelect { values } => selected_actions = values,
            // stop listening on the next button
            serenity::ComponentInteractionDataKind::Button => if interaction.data.custom_id == "next" {
                if !selected_actions.is_empty() {
                    complete = true;
                    break;
                }
                say_ephemeral(ctx, "Must specify at least one action", true).await?;
            }
            kind => {
                tracing::error!("Unexpected data from interaction {:?}", kind);
                return Err(InternalError::UnexpectedInteraction("Unexpected data from interaction".to_string()).into());
            }
        }

        // Can't use interaction.create_response due to lifetime issues
        serenity::CreateInteractionResponse::Acknowledge
            .execute(ctx.serenity_context(), (interaction.id, &interaction.token))
            .await?;

        let prefix = format!("What actions to manage for resource: {:?}", resource);
        handle.edit(
            ctx,
            poise::CreateReply::default()
                .content(build_prompt(prefix, &channels, &roles, &users)?)
                .components(vec![
                    serenity::CreateActionRow::SelectMenu(
                        serenity::CreateSelectMenu::new("actions", serenity::CreateSelectMenuKind::String {
                            options: actions.clone(),
                        })
                            .placeholder("Select actions")
                            .min_values(1)
                            .max_values(num_actions)
                    ),
                    serenity::CreateActionRow::Buttons(vec![
                        serenity::CreateButton::new("next")
                            .label("Next")
                            .style(serenity::ButtonStyle::Primary)
                            .disabled(!selected_actions.is_empty())
                    ]),
                ]),
        ).await?;
    }

    if !complete {
        return Err(InternalError::Timeout("waiting for action selection".to_string()).into());
    }

    let prefix = format!("Changing permissions for\nResource: {}/{:?}\nActions:{:?}", resource, specifier, selected_actions);
    let msg = build_prompt(prefix, &channels, &roles, &users)?;

    handle.edit(
        ctx,
        poise::CreateReply::default()
            .content(msg)
            .components(vec![])
    ).await?;

    Ok(())
}

const PROMPT_PREFIX: &str = "What principle to create policy for? \
Choose nothing for server-wide settings. \
When selecting multiple from each category (channel, role, user), policy will apply when at \
least one from each category matches";

fn build_prompt(prefix: impl Into<String>, channels: &[serenity::ChannelId], roles: &[serenity::RoleId], users: &[serenity::UserId]) -> std::result::Result<String, std::fmt::Error> {
    use std::fmt::Write as _;
    let mut prompt = prefix.into();
    if channels.is_empty() && roles.is_empty() && users.is_empty() {
        write!(&mut prompt, "\nServer-wide")?;
    }

    write_mentionables(&mut prompt, "Channels", channels)?;
    write_mentionables(&mut prompt, "Roles", roles)?;
    write_mentionables(&mut prompt, "Users", users)?;

    Ok(prompt)
}

// TODO can this be better? Causes lots of allocations
fn write_mentionables<T: serenity::Mentionable>(prompt: &mut impl std::fmt::Write, label: &str, slice: &[T]) -> std::fmt::Result {
    let mentions = slice
        .iter()
        .map(|c| c.mention().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    if !mentions.is_empty() {
        write!(prompt, "\n{}: {}", label, mentions)?;
    }

    Ok(())
}
