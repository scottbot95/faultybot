use poise::serenity_prelude as serenity;

const NEXT_BUTTON_ID: &str = "__poise_wizard_next";
const CANCEL_BUTTON_ID: &str = "__poise_wizard_cancel";

#[poise::async_trait]
pub trait WizardStep<U, E>: Send + Sync {
    type State:  Send + Sync;

    // fn create(input: Self::Input) -> Self;

    fn create_reply(&self, state: &Self::State) -> poise::CreateReply;

    fn is_valid(&self) -> bool {
        true
    }

    /// Handle an interaction.
    ///
    /// Should update any internal state based off user input.
    /// Returns true if the interaction was handled or false if the interaction was not recognized
    async fn handle_interaction(&mut self, ctx: poise::Context<'_, U, E>, interaction: serenity::ComponentInteraction) -> Result<bool, E>;

}

#[poise::async_trait]
pub trait Wizard<U, E>: {
    async fn execute<'a>(&mut self, ctx: poise::Context<'a, U, E>) -> Result<Option<poise::ReplyHandle<'a>>, E>;
}

fn add_step_change_buttons(mut builder: poise::CreateReply, can_proceed: bool) -> poise::CreateReply {
    let next_row = serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new(NEXT_BUTTON_ID)
            .label("Next")
            .style(serenity::ButtonStyle::Primary)
            .disabled(!can_proceed),
        serenity::CreateButton::new(CANCEL_BUTTON_ID)
            .label("Cancel")
            .style(serenity::ButtonStyle::Secondary)
    ]);

    if let Some(components) = &mut builder.components {
        components.push(next_row);
    } else {
        builder = builder.components(vec![next_row]);
    }

    builder
}

#[derive(derive_more::Display)]
enum StepStatus {
    Timeout,
    Cancel,
    Next,
}

pub async fn execute_step<'a, T, U: Sync, E>(
    step: &mut T,
    ctx: poise::Context<'a, U, E>,
    state: &T::State,
    previous: Option<poise::ReplyHandle<'a>>,
) -> Result<Option<poise::ReplyHandle<'a>>, E>
    where
        T: WizardStep<U, E> + Sized,
        U: Sync,
        E: From<serenity::Error>,
{
    use futures::StreamExt as _;

    let step_builder = add_step_change_buttons(
        step.create_reply(state),
        step.is_valid()
    );

    let step_handle = if let Some(handle) = previous {
        tracing::debug!("Editing previous step message");
        handle.edit(ctx, step_builder).await?;
        handle
    } else {
        tracing::debug!("Creating new message");
        ctx.send(step_builder)
            .await?
    };
    let step_message = step_handle.message().await?;

    let mut status = StepStatus::Timeout;

    let mut stream = step_message
        .await_component_interactions(ctx)
        .author_id(ctx.author().id)
        .timeout(std::time::Duration::from_secs(60 * 60))
        .stream();
    while let Some(interaction) = stream.next().await
    {
        interaction.create_response(
            ctx.serenity_context(),
            serenity::CreateInteractionResponse::Acknowledge
        ).await?;

        tracing::debug!("Handling interaction {:?}", &interaction);
        match (&interaction.data.kind, interaction.data.custom_id.as_str()) {
            (serenity::ComponentInteractionDataKind::Button, NEXT_BUTTON_ID) => {
                tracing::debug!("Next button clicked");
                status = StepStatus::Next;
                break;
            }
            (serenity::ComponentInteractionDataKind::Button, CANCEL_BUTTON_ID) => {
                tracing::debug!("Cancel button clicked");
                status = StepStatus::Cancel;
                break;
            }
            (_, id) => {
                let id_owned = id.to_owned();
                if !step.handle_interaction(ctx, interaction).await? {
                    tracing::error!("Unexpected data from interaction Id: {}", id_owned);
                    panic!("Unexpected data from interaction Id: {}", id_owned);
                }
            }
        }

        let reply = add_step_change_buttons(
            step.create_reply(state),
            step.is_valid()
        );

        tracing::debug!("Updating response");
        step_handle.edit(ctx, reply).await?;
    }

    tracing::debug!("Done with step: {}", status);

    match status {
        StepStatus::Timeout => {
            step_handle.edit(
                ctx,
                poise::CreateReply::default()
                    .content("Timeout waiting for user input")
                    .ephemeral(true)
            ).await?;
            Ok(None)
        }
        StepStatus::Cancel => {
            tracing::debug!("Deleting original message");
            step_handle.delete(ctx).await?;
            Ok(None)
        }
        StepStatus::Next => {
            Ok(Some(step_handle))
        }
    }
}