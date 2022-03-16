use std::fmt::{Display, Formatter};

use anyhow::Result;
use lazy_static::lazy_static;
use serenity::async_trait;
use google_sheets4::{Sheets, oauth2, hyper, hyper_rustls};

static DOC_LINK: &str = "https://docs.google.com/spreadsheets/d/1uZBSuuw_oxiR3Lr3MS8lNom2HlUz6_O0Nb6yZA0Vzy4/edit?usp=sharing";
lazy_static! {
    static ref CREDS_JSON_PATH: String = {
        std::env::var("CREDS_JSON_PATH")
            .expect("CREDS_JSON_PATH is a required environment variable")
    };
}

#[derive(Clone, Debug)]
pub struct Album {
    name: String,
    artist: String,
    genre: String,
    added_by: String,
}

impl Display for Album {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Album: {}, Artist: {}, Genre: {}, Added By: {}",
            self.name, self.artist, self.genre, self.added_by
        )
    }
}

#[async_trait]
pub trait AlbumRepo {
    async fn fetch_random_album(&self) -> Album;
}

pub struct GoogleSheetsAlbumRepo;

impl GoogleSheetsAlbumRepo {
    pub async fn default() -> Result<Self> {
        let service_account_key = oauth2::read_service_account_key(CREDS_JSON_PATH.clone()).await?;
        let auth = oauth2::ServiceAccountAuthenticator::builder(service_account_key)
        .build()
        .await
        .expect("failed to create authenticator");
        Ok(GoogleSheetsAlbumRepo)
    }
}

#[async_trait]
impl AlbumRepo for GoogleSheetsAlbumRepo {
    async fn fetch_random_album(&self) -> Album {
        Album {
            name: "Al's Bum".to_owned(),
            artist: "Arteest".to_owned(),
            genre: "Music".to_owned(),
            added_by: "Satan".to_owned(),
        }
    }
}

#[cfg(test)]
mod test {

    #[tokio::test]
    async fn test_getting_rows() {}
}
