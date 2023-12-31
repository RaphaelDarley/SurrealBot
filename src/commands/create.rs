use std::cmp::Ordering;

use serenity::model::prelude::application_command::{
    ApplicationCommandInteraction, CommandDataOptionValue,
};
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::{Guild, GuildChannel, PermissionOverwrite, UserId};
use serenity::model::Permissions;
use serenity::prelude::Context;
use serenity::{builder::CreateApplicationCommand, model::prelude::ChannelType};
use surrealdb::engine::local::Mem;
use surrealdb::Surreal;
use tokio::time::{sleep_until, Instant};

use crate::{db_utils::*, ConnType};
use crate::{premade, utils::*};

use crate::config::Config;
use crate::utils::{interaction_reply, interaction_reply_edit, interaction_reply_ephemeral};
use crate::{DB, DBCONNS};

pub async fn run(
    command: &ApplicationCommandInteraction,
    ctx: Context,
) -> Result<(), anyhow::Error> {
    match command.guild_id {
        Some(id) => {
            let result: Result<Option<Config>, surrealdb::Error> =
                DB.select(("guild_config", id.to_string())).await;

            let config = match result {
                Ok(response) => {
                    match response {
                        Some(c) => {c}
                        None => return interaction_reply_ephemeral(command, ctx, "No config found for this server, please ask an administrator to configure the bot".to_string()).await
                    }
                }
                Err(e) => return interaction_reply(command, ctx, format!("Database error: {}", e)).await,
            };

            println!("options array length:{:?}", command.data.options.len());

            let guild = Guild::get(&ctx, id).await.unwrap();

            let everyone = guild.role_by_name("@everyone");

            let perms = vec![
                PermissionOverwrite {
                    allow: Permissions::empty(),
                    deny: Permissions::VIEW_CHANNEL,
                    kind: serenity::model::prelude::PermissionOverwriteType::Role(
                        everyone.unwrap().id,
                    ),
                },
                PermissionOverwrite {
                    allow: Permissions::VIEW_CHANNEL
                        .union(Permissions::SEND_MESSAGES)
                        .union(Permissions::READ_MESSAGE_HISTORY),
                    deny: Permissions::empty(),
                    kind: serenity::model::prelude::PermissionOverwriteType::Member(UserId(
                        command.application_id.as_u64().clone(),
                    )),
                },
                PermissionOverwrite {
                    allow: Permissions::VIEW_CHANNEL
                        .union(Permissions::SEND_MESSAGES)
                        .union(Permissions::READ_MESSAGE_HISTORY),
                    deny: Permissions::empty(),
                    kind: serenity::model::prelude::PermissionOverwriteType::Member(
                        command.user.id,
                    ),
                },
            ];

            let channel = guild
                .create_channel(&ctx, |c| {
                    c.name(command.id.to_string())
                        .kind(ChannelType::Text)
                        .category(config.active_channel)
                        .permissions(perms)
                })
                .await
                .unwrap();
            let db = Surreal::new::<Mem>(()).await.unwrap();
            db.use_ns("test").use_db("test").await.unwrap();

            match command.data.options.len().cmp(&1) {
                Ordering::Greater => {
                    interaction_reply_ephemeral(command, ctx, "Please only supply one arguement (you can use the up arrow to edit the previous command)").await?;
                    return Ok(());
                }
                Ordering::Equal => {
                    let op_option = command.data.options[0].clone();
                    match op_option.kind {
                        CommandOptionType::String => {
                            match op_option.value.unwrap().as_str().unwrap() {
                                "surreal_deal_mini" => {
                                    interaction_reply(command, ctx.clone(), format!("You now have your own database instance, head over to <#{}> data is currently being loaded, soon you'll be able to query the surreal deal(mini) dataset!!!", channel.id.as_u64())).await?;
                                    let db = db.clone();
                                    let (channel, ctx, command) =
                                        (channel.clone(), ctx.clone(), command.clone());
                                    tokio::spawn(async move {
                                        channel.say(&ctx, format!("This channel is now connected to a SurrealDB instance which is loading data, it will be ready to query soon!\n(note this will expire in {:#?}", config.ttl)).await.unwrap();
                                        db.import("premade/surreal_deal_mini.surql").await.unwrap();
                                        channel.say(&ctx, format!("<@{}>This channel is now connected to a SurrealDB instance with the surreal deal(mini) dataset, try writing some SurrealQL!!!", command.user.id.as_u64())).await.unwrap();
                                    });
                                }
                                "surreal_deal" => {
                                    interaction_reply(command, ctx.clone(), format!("You now have your own database instance, head over to <#{}> data is currently being loaded, soon you'll be able to query the surreal deal dataset!!!", channel.id.as_u64())).await?;
                                    let db = db.clone();
                                    let (channel, ctx, command) =
                                        (channel.clone(), ctx.clone(), command.clone());
                                    tokio::spawn(async move {
                                        channel.say(&ctx, format!("This channel is now connected to a SurrealDB instance which is loading data, it will be ready to query soon!\n(note this will expire in {:#?}", config.ttl)).await.unwrap();
                                        db.import("premade/surreal_deal.surql").await.unwrap();
                                        channel.say(&ctx, format!("<@{}>This channel is now connected to a SurrealDB instance with the surreal deal dataset, try writing some SurrealQL!!!", command.user.id.as_u64())).await.unwrap();
                                    });
                                }
                                _ => {
                                    println!("wildcard hit");
                                    interaction_reply_ephemeral(
                                        command,
                                        ctx,
                                        "Cannot find requested dataset",
                                    )
                                    .await?;
                                    return Ok(());
                                }
                            }
                        }
                        CommandOptionType::Attachment => {
                            // let val = op_option.resolved.unwrap();
                            if let Some(CommandDataOptionValue::Attachment(attachment)) =
                                op_option.resolved
                            {
                                interaction_reply(command, ctx.clone(), format!("You now have your own database instance, head over to <#{}> your file is now being downloaded!!!", channel.id.as_u64())).await?;
                                match attachment.download().await {
                                    Ok(data) => {
                                        interaction_reply_edit(command, ctx.clone(), format!("You now have your own database instance, head over to <#{}> data is currently being loaded, soon you'll be able to query your dataset!!!", channel.id.as_u64())).await?;
                                        println!("attachment downloaded");

                                        let db = db.clone();
                                        let (channel, ctx, command) =
                                            (channel.clone(), ctx.clone(), command.clone());
                                        tokio::spawn(async move {
                                            channel.say(&ctx, format!("This channel is now connected to a SurrealDB instance which is loading data, it will be ready to query soon!\n(note this will expire in {:#?}", config.ttl)).await.unwrap();
                                            db.query(String::from_utf8_lossy(&data).into_owned())
                                                .await
                                                .unwrap();
                                            channel.say(&ctx, format!("<@{}>This channel is now connected to a SurrealDB instance with your dataset, try writing some SurrealQL!!!", command.user.id.as_u64())).await.unwrap();
                                            interaction_reply_edit(
                                                &command,
                                                ctx,
                                                format!("You now have your own database instance, head over to <#{}> to start writing SurrealQL to query your data!!!", channel.id.as_u64()),
                                            )
                                            .await
                                            .unwrap();
                                        });
                                    }
                                    Err(why) => {
                                        interaction_reply_edit(
                                            command,
                                            ctx,
                                            format!("Error with attachment: {}", why),
                                        )
                                        .await?;
                                        return Ok(());
                                    }
                                }
                            } else {
                                interaction_reply_edit(command, ctx, "Error with attachment")
                                    .await?;
                                return Ok(());
                            }
                        }
                        _ => {
                            interaction_reply_ephemeral(command, ctx, "Unsupported option type")
                                .await?;
                            return Ok(());
                        }
                    }
                }
                Ordering::Less => {
                    channel.say(&ctx, format!("This channel is now connected to a SurrealDB instance, try writing some SurrealQL!!!\n(note this will expire in {:#?})", config.ttl)).await?;
                    interaction_reply(command, ctx.clone(), format!("You now have your own database instance, head over to <#{}> to start writing SurrealQL!!!", channel.id.as_u64())).await?;
                }
            };

            DBCONNS.lock().await.insert(
                channel.id.as_u64().clone(),
                crate::Conn {
                    db: db,
                    last_used: Instant::now(),
                    conn_type: ConnType::Channel,
                    ttl: config.ttl.clone(),
                    pretty: config.pretty.clone(),
                    json: config.json.clone(),
                },
            );

            tokio::spawn(async move {
                let mut last_time;
                let mut ttl;
                loop {
                    match DBCONNS.lock().await.get(channel.id.as_u64()) {
                        Some(e) => {
                            last_time = e.last_used;
                            ttl = e.ttl
                        }
                        None => {
                            clean_channel(channel, &ctx).await;
                            break;
                        }
                    }
                    if last_time.elapsed() >= ttl {
                        clean_channel(channel, &ctx).await;
                        break;
                    }
                    sleep_until(last_time + ttl).await;
                }
            });
            return Ok(());
        }
        None => {
            return interaction_reply(
                command,
                ctx,
                "Direct messages are not currently supported".to_string(),
            )
            .await;
        }
    }
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("create")
        .description("Creates a channel with a SurrealDB instance")
        .create_option(premade::register)
        .create_option(|option| {
            option
                .name("file")
                .description("a SurrealQL to load into the database instance")
                .kind(CommandOptionType::Attachment)
                .required(false)
        })
}
