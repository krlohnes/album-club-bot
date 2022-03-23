use std::collections::HashSet;
use std::fmt::{Display, Formatter};

use anyhow::{anyhow, Result};
use google_sheets4::{hyper, hyper_rustls, oauth2, Sheets};
use lazy_static::lazy_static;
use rand::Rng;
use serenity::async_trait;

lazy_static! {
    static ref CREDS_JSON_PATH: String = {
        std::env::var("CREDS_JSON_PATH")
            .expect("CREDS_JSON_PATH is a required environment variable")
    };
    static ref DOC_ID: String = {
        std::env::var("SHEET_ID_ALBUM_BOT")
            .expect("SHEET_ID_ALBUM_BOT is a required environment variable")
    };
}

const GET_ALBUMS_RANGE: &str = "Album Selection!A2:D";
const GET_ROTATION_RANGE: &str = "Rotation!A1:A4";
const GET_LAST_GENRE_RANGE: &str = "Ratings!C2:D2";

#[derive(Clone, Debug)]
pub struct Album {
    pub name: String,
    pub artist: String,
    pub genre: String,
    pub added_by: String,
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
    async fn fetch_random_album(&self) -> Result<Album>;
}

pub struct GoogleSheetsAlbumRepo {
    hub: Sheets,
}

impl GoogleSheetsAlbumRepo {
    pub async fn default() -> Result<Self> {
        let service_account_key = oauth2::read_service_account_key(CREDS_JSON_PATH.clone()).await?;
        let auth = oauth2::ServiceAccountAuthenticator::builder(service_account_key)
            .build()
            .await
            .expect("failed to create authenticator");
        let hub = Sheets::new(
            hyper::Client::builder().build(hyper_rustls::HttpsConnector::with_native_roots()),
            auth,
        );
        Ok(GoogleSheetsAlbumRepo { hub })
    }

    async fn get_last_genre_and_added_by(&self) -> Result<(String, String)> {
        let (_, spreadsheet) = self
            .hub
            .spreadsheets()
            .values_get(&DOC_ID, GET_LAST_GENRE_RANGE)
            .doit()
            .await?;
        let row: Vec<String> = spreadsheet
            .values
            .as_ref()
            .ok_or_else(|| anyhow!("Unable to get last genre"))?
            .iter()
            .flatten()
            .map(|x| x.to_owned())
            .collect::<Vec<String>>();
        let genre = row
            .get(0)
            .ok_or_else(|| anyhow!("Error getting last genre"))?;
        let selected_by = row
            .get(1)
            .ok_or_else(|| anyhow!("Error getting last genre"))?;
        Ok((genre.to_owned(), selected_by.to_owned()))
    }

    async fn get_rotation(&self) -> Result<HashSet<String>> {
        let (_, spreadsheet) = self
            .hub
            .spreadsheets()
            .values_get(&DOC_ID, GET_ROTATION_RANGE)
            .doit()
            .await?;
        let names: HashSet<String> = HashSet::from_iter(
            spreadsheet
                .values
                .ok_or_else(|| "Error getting rotation")
                .into_iter()
                .flatten()
                .flatten(),
        );
        Ok(names)
    }

    async fn select_random_album(&self, spreadsheet: &Vec<Vec<String>>) -> Result<Album> {
        let row_count = spreadsheet.len();
        let num = rand::thread_rng().gen_range(0..row_count);
        let values = spreadsheet
            .get(num)
            .ok_or_else(|| anyhow!("Error getting cell values"))?
            .to_owned();

        let artist = values
            .get(0)
            .ok_or_else(|| anyhow!("Unable to get album artist"))?
            .to_owned();

        let name = values
            .get(1)
            .ok_or_else(|| anyhow!("Unable to get album name"))?
            .to_owned();

        let genre = values
            .get(2)
            .ok_or_else(|| anyhow!("Unable to get album genre"))?
            .to_owned();

        let added_by = values
            .get(3)
            .ok_or_else(|| anyhow!("Unable to get album added_by"))?
            .to_owned();

        let random_album = Album {
            name,
            artist,
            genre,
            added_by,
        };
        Ok(random_album)
    }
}

#[async_trait]
impl AlbumRepo for GoogleSheetsAlbumRepo {
    async fn fetch_random_album(&self) -> Result<Album> {
        let (_, spreadsheet) = self
            .hub
            .spreadsheets()
            .values_get(&DOC_ID, GET_ALBUMS_RANGE)
            .doit()
            .await?;
        let albums = &spreadsheet
            .values
            .ok_or_else(|| anyhow!("Error fetching albums"))?;
        let rotation = self.get_rotation().await?;
        let album: Album;
        let (last_genre, last_added_by) = self.get_last_genre_and_added_by().await?;

        loop {
            let try_album = self.select_random_album(albums).await?;
            if !rotation.contains(&try_album.added_by)
                && try_album.genre.as_str().to_lowercase() != last_genre.as_str().to_lowercase()
                && try_album.added_by.as_str().to_lowercase()
                    != last_added_by.as_str().to_lowercase()
            {
                album = try_album;
                break;
            }
        }
        Ok(album)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    #[allow(dead_code)]
    async fn test_getting_rotation() -> Result<()> {
        env_logger::init();
        let repo = GoogleSheetsAlbumRepo::default().await?;

        let album = match repo.get_last_genre_and_added_by().await {
            Ok(a) => a,
            Err(e) => {
                println!("{:?}", e);
                return Err(e);
            }
        };
        println!("{:?}", album);
        Ok(())
    }

    //#[tokio::test]
    #[allow(dead_code)]
    async fn test_getting_rows() -> Result<()> {
        env_logger::init();
        let repo: Box<dyn AlbumRepo> = Box::new(GoogleSheetsAlbumRepo::default().await?);

        let album = match repo.fetch_random_album().await {
            Ok(a) => a,
            Err(e) => {
                println!("{:?}", e);
                return Err(e);
            }
        };
        println!("{}", album);
        Ok(())
    }
}
