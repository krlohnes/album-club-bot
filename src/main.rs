mod albums;

use std::env;

use crate::albums::{AlbumRepo, GoogleSheetsAlbumRepo};

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
        if let Some(_) = msg.content.strip_prefix("~album next") {
            let album = match self.album_repo.fetch_random_album().await {
                Ok(album) => album,
                Err(e) => {
                    error!("Error getting a random album {:?}", e);
                    msg.channel_id
                        .say(&ctx, ERROR_RESPONSE_FETCH_RANDOM.clone())
                        .await
                        .map_err(|e| error!("Error sending message back to channel {:?}", e))
                        .ok();
                    return;
                }
            };
            let response = format!("The next album is {}", album);
            msg.channel_id.say(&ctx, response).await.unwrap();
        }
    }
}

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
