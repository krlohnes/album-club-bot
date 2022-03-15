mod albums;

use std::env;

use crate::albums::{AlbumRepo, GoogleSheetsAlbumRepo};

use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::framework::standard::{macros::group, StandardFramework};
use serenity::model::channel::Message;

#[group]
struct General;

struct AlbumHandler {
    album_repo: Box<dyn AlbumRepo + Send + Sync>,
}

#[async_trait]
impl EventHandler for AlbumHandler {
    async fn message(&self, ctx: Context, msg: Message) {
        if let Some(_) = msg.content.strip_prefix("~album next") {
            let album = self.album_repo.fetch_random_album().await;
            let response = format!("The next album is {}", album);
            msg.channel_id.say(&ctx, response).await.unwrap();
        }
    }
}

#[tokio::main]
async fn main() {
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
        .group(&GENERAL_GROUP);

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("token");
    let handler = AlbumHandler {
        album_repo: Box::new(GoogleSheetsAlbumRepo::default()),
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
