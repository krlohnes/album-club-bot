mod albums;
mod spotify;

use std::env;

use crate::albums::{AlbumRepo, GoogleSheetsAlbumRepo};
use crate::spotify::Spotify;

use log::error;
use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::framework::standard::{macros::group, StandardFramework};
use serenity::model::application::command::CommandOptionType;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::gateway::GatewayIntents;
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;

#[group]
struct General;

struct AlbumHandler {
    album_repo: Box<dyn AlbumRepo + Send + Sync>,
}

const ERROR_RESPONSE_FETCH_RANDOM: &str = "Try again later!";
const WE_HAVE_OPTIONS_FOR_A_REASON: &str = "C'mon folks, use the options for the slash command!";

impl AlbumHandler {
    async fn get_next_reviewer(&self) -> String {
        match self.album_repo.get_random_name().await {
            Ok(person) => format!("The next reviewer is {}", person),
            Err(e) => {
                error!("Error getting a random person {:?}", e);
                ERROR_RESPONSE_FETCH_RANDOM.to_owned()
            }
        }
    }

    async fn reset_reviewers(&self) -> String {
        match self.album_repo.reset_reviewers().await {
            Ok(_) => "Reviewer list has been reset".to_owned(),
            Err(e) => {
                error!("Error resetting reviewer {:?}", e);
                ERROR_RESPONSE_FETCH_RANDOM.to_owned()
            }
        }
    }

    async fn get_current_album(&self) -> String {
        let album = match self.album_repo.get_current().await {
            Ok(album) => album,
            Err(_) => {
                return ERROR_RESPONSE_FETCH_RANDOM.to_owned();
            }
        };
        let url = Spotify::fetch_album_link(&album)
            .await
            .map_err(|e| error!("Error getting spotify url {:?}", e))
            .ok();
        if let Some(Some(url)) = url {
            return format!("The current album is {} \n {}", album, url);
        } else {
            return format!(
                "The current album is {} \n I had trouble finding the album on spotify",
                album
            );
        }
    }

    async fn get_next_album(&self) -> String {
        let album = match self.album_repo.fetch_random_album().await {
            Ok(album) => album,
            Err(e) => {
                error!("Error getting a random album {:?}", e);
                return ERROR_RESPONSE_FETCH_RANDOM.to_owned();
            }
        };
        let url = Spotify::fetch_album_link(&album)
            .await
            .map_err(|e| error!("Error getting spotify url {:?}", e))
            .ok();
        match url {
            Some(url) => {
                if let Some(url) = url {
                    format!("The next album is {} \n {}", album, url)
                } else {
                    format!(
                        "The next album is {} \n I had some trouble finding it on Spotify though.",
                        album
                    )
                }
            }
            None => {
                format!(
                    "The next album is {} \n I had some trouble finding the album on Spotify though.",
                    album
                )
            }
        }
    }
}

#[async_trait]
impl EventHandler for AlbumHandler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let content = match command.data.name.as_str() {
                "album" => {
                    let result = match command.data.options.get(0) {
                        Some(option) => {
                            match option.value.as_ref().unwrap().as_str().unwrap().as_ref() {
                                "next" => self.get_next_album().await,
                                "current" => self.get_current_album().await,
                                e => {
                                    error!("Got command {:?}", e);
                                    WE_HAVE_OPTIONS_FOR_A_REASON.to_owned()
                                }
                            }
                        }
                        None => WE_HAVE_OPTIONS_FOR_A_REASON.to_owned(),
                    };
                    result
                }
                "reviewer" => {
                    let result = match command.data.options.get(0) {
                        Some(option) => {
                            match option.value.as_ref().unwrap().as_str().unwrap().as_ref() {
                                "next" => self.get_next_reviewer().await,
                                "reset" => self.reset_reviewers().await,
                                _ => WE_HAVE_OPTIONS_FOR_A_REASON.to_owned(),
                            }
                        }
                        None => WE_HAVE_OPTIONS_FOR_A_REASON.to_owned(),
                    };
                    result
                }
                _ => "Go home, you're drunk :(".to_string(),
            };

            if let Err(why) = command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| message.content(content))
                })
                .await
            {
                error!("Cannot respond to slash command: {}", why);
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let guild_id = GuildId(
            env::var("GUILD_ID")
                .expect("Expected GUILD_ID in environment")
                .parse()
                .expect("GUILD_ID must be an integer"),
        );

        let _ = GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
            commands
                .create_application_command(|command| {
                    command
                        .name("reviewer")
                        .description("A slash command for getting or resetting reviewers")
                        .create_option(|option| {
                            option
                                .name("command")
                                .description("What command you want for reviewers")
                                .kind(CommandOptionType::String)
                                .required(true)
                                .add_string_choice("Get the next one", "next")
                                .add_string_choice("Reset the list", "reset")
                        })
                })
                .create_application_command(|command| {
                    command
                        .name("album")
                        .description("A slash command for getting the next or current album")
                        .create_option(|option| {
                            option
                                .name("command")
                                .description("What action you want to take for albums")
                                .kind(CommandOptionType::String)
                                .required(true)
                                .add_string_choice("Get the next one", "next")
                                .add_string_choice("Get the current one", "current")
                        })
                })
        })
        .await;
    }
}

//https://discordapp.com/oauth2/authorize?client_id=%954535768796307527%3e&scope=bot&permissions=2147483648
#[tokio::main]
async fn main() {
    env_logger::init();
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
        .group(&GENERAL_GROUP);

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("token");
    let handler = AlbumHandler {
        album_repo: Box::new(GoogleSheetsAlbumRepo::default().await.unwrap()),
    };

    let mut client = Client::builder(token, GatewayIntents::empty())
        .event_handler(handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}
