use std::fmt::{Display, Formatter};

use lazy_static::lazy_static;
use serenity::async_trait;

static DOC_LINK: &str = "https://docs.google.com/spreadsheets/d/1uZBSuuw_oxiR3Lr3MS8lNom2HlUz6_O0Nb6yZA0Vzy4/edit?usp=sharing";
lazy_static! {
    static ref CLIENT_ID: String = {
        std::env::var("GAPI_CLIENT_ID").expect("GAPI_CLIENT_ID is a required environment variable")
    };

    static ref KEY: String = {
        std::env::var("GAPI_KEY").expect("GAPI_KEY is a required environment variable")
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
    pub fn default() -> Self {
        GoogleSheetsAlbumRepo
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
    async fn test_getting_rows() {

    }

}
