mod albums;
mod spotify;

use std::env;

use crate::albums::{AlbumRepo, GoogleSheetsAlbumRepo};
use crate::spotify::Spotify;

use log::error;
use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::framework::standard::{macros::group, StandardFramework};
use serenity::model::channel::Message;

#[group]
struct General;

struct AlbumHandler {
    album_repo: Box<dyn AlbumRepo + Send + Sync>,
}

const ERROR_RESPONSE_FETCH_RANDOM: &str = "Try again later!";

#[async_trait]
impl EventHandler for AlbumHandler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content.strip_prefix("~album next").is_some() {
            let album = match self.album_repo.fetch_random_album().await {
                Ok(album) => album,
                Err(e) => {
                    error!("Error getting a random album {:?}", e);
                    msg.channel_id
                        .say(&ctx, &(*ERROR_RESPONSE_FETCH_RANDOM))
                        .await
                        .map_err(|e| error!("Error sending message back to channel {:?}", e))
                        .ok();
                    return;
                }
            };
            let response = format!("The next album is {}", album);
            msg.channel_id.say(&ctx, response).await.ok();
            let url = Spotify::fetch_album_link(&album)
                .await
                .map_err(|e| error!("Error getting spotify url {:?}", e))
                .ok();
            match url {
                Some(url) => {
                    if let Some(url) = url {
                        msg.channel_id.say(&ctx, url).await.ok();
                    } else {
                        msg.channel_id
                            .say(
                                &ctx,
                                "I had some trouble trying to find the album on spotify",
                            )
                            .await
                            .ok();
                    }
                }
                None => {
                    msg.channel_id
                        .say(
                            &ctx,
                            "I had some trouble trying to find the album on spotify",
                        )
                        .await
                        .ok();
                }
            }
        } else if msg.content.strip_prefix("~album current").is_some() {
            let album = match self.album_repo.get_current().await {
                Ok(album) => album,
                Err(e) => {
                    error!("Error getting a current album {:?}", e);
                    msg.channel_id
                        .say(&ctx, &(*ERROR_RESPONSE_FETCH_RANDOM))
                        .await
                        .map_err(|e| error!("Error sending message back to channel {:?}", e))
                        .ok();
                    return;
                }
            };
            let response = format!("The current album is {}", album);
            msg.channel_id.say(&ctx, response).await.ok();
            //TODO DRY this out.
            let url = Spotify::fetch_album_link(&album)
                .await
                .map_err(|e| error!("Error getting spotify url {:?}", e))
                .ok();
            match url {
                Some(url) => {
                    if let Some(url) = url {
                        msg.channel_id.say(&ctx, url).await.ok();
                    } else {
                        msg.channel_id
                            .say(
                                &ctx,
                                "I had some trouble trying to find the album on spotify",
                            )
                            .await
                            .ok();
                    }
                }
                None => {
                    msg.channel_id
                        .say(
                            &ctx,
                            "I had some trouble trying to find the album on spotify",
                        )
                        .await
                        .ok();
                }
            }
        } else if msg.content.strip_prefix("~reviewer").is_some() {
            let person = match self.album_repo.get_random_name().await {
                Ok(person) => person,
                Err(e) => {
                    error!("Error getting a random person {:?}", e);
                    msg.channel_id
                        .say(&ctx, &(*ERROR_RESPONSE_FETCH_RANDOM))
                        .await
                        .map_err(|e| error!("Error sending message back to channel {:?}", e))
                        .ok();
                    return;
                }
            };

            msg.channel_id
                .say(&ctx, format!("Next reviewer is {}", person))
                .await
                .ok();
        }
    }
}

//https://discordapp.com/oauth2/authorize?client_id=%3cBot_Client_ID%3e&scope=bot&permissions=0
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
    let mut client = Client::builder(token)
        .event_handler(handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}
