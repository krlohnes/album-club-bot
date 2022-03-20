use std::collections::HashSet;
use std::fmt::{Display, Formatter};

use anyhow::{anyhow, Result};
use google_sheets4::api::{CellData, DataFilter, GetSpreadsheetByDataFilterRequest, Spreadsheet};
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
    static ref GET_LAST_GENRE: DataFilter = DataFilter {
        a1_range: Some("Ratings!C2".to_owned()),
        developer_metadata_lookup: None,
        grid_range: None,
    };
    static ref GET_LAST_GENRE_REQUEST: GetSpreadsheetByDataFilterRequest =
        GetSpreadsheetByDataFilterRequest {
            data_filters: Some(vec![GET_LAST_GENRE.clone()]),
            include_grid_data: Some(true),
        };
}

const GET_ALBUMS_RANGE: &str = "Album Selection!A2:D";
const GET_ROTATION_RANGE: &str = "Rotation!A1:A4";

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

    async fn get_last_genre(&self) -> Result<String> {
        let (_, spreadsheet) = self
            .hub
            .spreadsheets()
            .get_by_data_filter(GET_LAST_GENRE_REQUEST.clone(), &DOC_ID)
            .doit()
            .await?;
        let genre = spreadsheet
            .sheets
            .as_ref()
            .and_then(|sheets| sheets.get(0))
            .and_then(|sheet| sheet.data.as_ref())
            .and_then(|data| data.get(0))
            .and_then(|row| row.row_data.as_ref())
            .and_then(|row_data| row_data.get(0))
            .and_then(|cells| cells.values.as_ref())
            .and_then(|cell_values| cell_values.get(0))
            .and_then(|cell_value| cell_value.effective_value.as_ref())
            .and_then(|effective_value| effective_value.string_value.to_owned())
            .ok_or_else(|| anyhow!("Unable to get last genre"))?;
        Ok(genre)
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

    async fn get_value_from_cell_data(
        &self,
        cell_position: usize,
        cell_data: &Vec<CellData>,
        error_msg: String,
    ) -> Result<String> {
        cell_data
            .get(cell_position)
            .and_then(|value| value.effective_value.as_ref())
            .and_then(|effective_value| effective_value.string_value.as_ref())
            .and_then(|string_value| Some(string_value.to_owned()))
            .ok_or_else(|| anyhow!(error_msg))
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
        let last_genre = self.get_last_genre().await?;

        loop {
            let try_album = self.select_random_album(albums).await?;
            if !rotation.contains(&try_album.added_by) && try_album.genre != last_genre {
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

    //#[tokio::test]
    #[allow(dead_code)]
    async fn test_getting_rotation() -> Result<()> {
        env_logger::init();
        let repo = GoogleSheetsAlbumRepo::default().await?;

        let album = match repo.get_rotation().await {
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
