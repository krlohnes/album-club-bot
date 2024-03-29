use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use google_sheets4::api::{ClearValuesRequest, ValueRange};
use google_sheets4::{hyper, hyper_rustls, oauth2, Sheets};
use lazy_static::lazy_static;
use rand::seq::SliceRandom;
use rand::Rng;
use serenity::async_trait;
use tokio::sync::Mutex;

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
const GET_ROTATION_RANGE: &str = "Rotation!A1:A10";
const GET_NAMES: &str = "Rotation!B1:B10";
const GET_LAST_GENRE_RANGE: &str = "Ratings!C2:D2";
const GET_CURRENT_RANGE: &str = "Ratings!A2:D2";

#[derive(Clone, Debug)]
pub struct Album {
    pub name: String,
    pub artist: String,
    pub genre: String,
    pub added_by: String,
    pub row: usize,
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
    async fn get_current(&self) -> Result<Album>;
    async fn get_random_name(&self) -> Result<String>;
    async fn reset_reviewers(&self) -> Result<()>;
    async fn add_name_to_rotation(&self, name: String) -> Result<()>;
}

pub struct GoogleSheetsAlbumRepo {
    hub: Sheets,
    persons: Arc<Mutex<Vec<String>>>,
}

impl GoogleSheetsAlbumRepo {
    pub async fn default() -> Result<Self> {
        let service_account_key = oauth2::read_service_account_key(CREDS_JSON_PATH.clone()).await?;
        let auth = oauth2::ServiceAccountAuthenticator::builder(service_account_key)
            .build()
            .await
            .expect("failed to create authenticator");
        let hub = Sheets::new(
            hyper::Client::builder().build(
                hyper_rustls::HttpsConnectorBuilder::new()
                    .with_native_roots()
                    .https_or_http()
                    .enable_http1()
                    .enable_http2()
                    .build(),
            ),
            auth,
        );
        return Ok(GoogleSheetsAlbumRepo {
            hub,
            persons: Arc::new(Mutex::new(Vec::new())),
        });
    }

    async fn album_from_vec(&self, values: &Vec<String>, row: usize) -> Result<Album> {
        if values.is_empty() {
            Err(anyhow!("No albums found"))
        } else {
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

            Ok(Album {
                name,
                artist,
                genre,
                added_by,
                row,
            })
        }
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

    async fn get_column_strings_as_hashset(&self, range: &str) -> Result<HashSet<String>> {
        let (_, spreadsheet) = self
            .hub
            .spreadsheets()
            .values_get(&DOC_ID, range)
            .doit()
            .await?;
        let names: HashSet<String> = HashSet::from_iter(
            spreadsheet
                .values
                .ok_or("Error getting rotation")
                .into_iter()
                .flatten()
                .flatten(),
        );
        Ok(names)
    }

    async fn get_names(&self) -> Result<HashSet<String>> {
        self.get_column_strings_as_hashset(GET_NAMES).await
    }

    async fn get_rotation(&self) -> Result<HashSet<String>> {
        self.get_column_strings_as_hashset(GET_ROTATION_RANGE).await
    }

    async fn is_full_rotation(&self, rotation: HashSet<String>) -> Result<bool> {
        let names = self.get_names().await?;
        for name in names {
            if !rotation.contains(&name) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn clear_rotation(&self) -> Result<()> {
        let req = ClearValuesRequest::default();
        self.hub
            .spreadsheets()
            .values_clear(req, &DOC_ID, GET_ROTATION_RANGE)
            .doit()
            .await?;
        Ok(())
    }

    async fn select_random_album(
        &self,
        spreadsheet: &[Vec<String>],
        rotation: &HashSet<String>,
        last_genre: &str,
        last_added_by: &str,
    ) -> Result<Album> {
        let mut i = 0;
        let mut filtered_albums = Vec::new();
        for x in spreadsheet {
            let album = self.album_from_vec(x, i).await?;
            i += 1;
            if !rotation.contains(&album.added_by)
                && &album.added_by.to_lowercase() != &last_added_by.to_lowercase()
                && &album.genre.to_lowercase() != &last_genre.to_lowercase()
            {
                filtered_albums.push(album)
            }
        }
        let row_count = filtered_albums.len();
        let num = rand::thread_rng().gen_range(0..row_count);
        Ok(filtered_albums[num].to_owned())
    }
}

#[async_trait]
impl AlbumRepo for GoogleSheetsAlbumRepo {
    async fn add_name_to_rotation(&self, name: String) -> Result<()> {
        let value_range = ValueRange {
            major_dimension: Some("COLUMNS".to_string()),
            range: Some(GET_ROTATION_RANGE.to_string()),
            values: Some(vec![vec![name.to_owned()]]),
        };
        self.hub
            .spreadsheets()
            .values_append(value_range, &DOC_ID, GET_ROTATION_RANGE)
            .value_input_option("RAW")
            .doit()
            .await?;
        let mut rotation = self.get_rotation().await?;
        rotation.insert(name);
        if self.is_full_rotation(rotation).await? {
            self.clear_rotation().await?;
        }
        Ok(())
    }

    async fn reset_reviewers(&self) -> Result<()> {
        let mut lock = self.persons.lock().await;
        let current_album = self.get_current().await?;
        *lock = self
            .get_names()
            .await?
            .into_iter()
            .filter(|name| name != &current_album.added_by)
            .collect();
        Ok(())
    }

    async fn get_random_name(&self) -> Result<String> {
        let mut lock = self.persons.lock().await;
        if lock.len() == 0 {
            let current_album = self.get_current().await?;
            *lock = self
                .get_names()
                .await?
                .into_iter()
                .filter(|name| name != &current_album.added_by)
                .collect();
            lock.shuffle(&mut rand::thread_rng());
        }
        Ok(lock.remove(0))
    }
    async fn get_current(&self) -> Result<Album> {
        let (_, spreadsheet) = self
            .hub
            .spreadsheets()
            .values_get(&DOC_ID, GET_CURRENT_RANGE)
            .doit()
            .await?;
        let row: Vec<String> = spreadsheet
            .values
            .as_ref()
            .ok_or_else(|| anyhow!("Unable to get current album"))?
            .iter()
            .flatten()
            .map(|x| x.to_owned())
            .collect::<Vec<String>>();
        self.album_from_vec(&row, 0).await
    }

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
        let (last_genre, last_added_by) = self.get_last_genre_and_added_by().await?;
        let album = self
            .select_random_album(albums, &rotation, &last_genre, &last_added_by)
            .await?;
        Ok(album)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    //#[tokio::test]
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
