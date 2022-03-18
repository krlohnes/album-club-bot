use std::fmt::{Display, Formatter};

use anyhow::{anyhow, Result};
use google_sheets4::api::{DataFilter, GetSpreadsheetByDataFilterRequest, Spreadsheet};
use google_sheets4::{hyper, hyper_rustls, oauth2, Sheets};
use lazy_static::lazy_static;
use rand::Rng;
use serenity::async_trait;

const DOC_ID: &str = "1uZBSuuw_oxiR3Lr3MS8lNom2HlUz6_O0Nb6yZA0Vzy4";
lazy_static! {
    static ref CREDS_JSON_PATH: String = {
        std::env::var("CREDS_JSON_PATH")
            .expect("CREDS_JSON_PATH is a required environment variable")
    };
}

lazy_static! {
    static ref GET_ALBUMS: DataFilter = DataFilter {
        a1_range: Some("Album Selection!A2:D".to_owned()),
        developer_metadata_lookup: None,
        grid_range: None,
    };
    static ref GET_ALBUMS_REQUEST: GetSpreadsheetByDataFilterRequest =
        GetSpreadsheetByDataFilterRequest {
            data_filters: Some(vec![GET_ALBUMS.clone()]),
            include_grid_data: None,
        };
}

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

    async fn select_random_album(&self, spreadsheet: Spreadsheet) -> Result<Album> {
        let sheets = spreadsheet.sheets.ok_or_else(|| anyhow!("No sheets on spreadsheet"))?;
        let sheet = sheets.get(0).ok_or_else(|| anyhow!("Empty vec of sheets"))?;
        let data = sheet.data.as_ref().ok_or_else(|| anyhow!("Error getting sheet data"))?;
        let data = data.get(0).ok_or_else(|| anyhow!("Error parsing sheet, unexpected data length"))?;
        let row_data = data.row_data.as_ref().ok_or_else(|| anyhow!("Row data does not exist"))?;
        let row_count = row_data.len();
        let num = rand::thread_rng().gen_range(0..row_count);
        let rand_row_data = row_data.get(num).ok_or_else(|| anyhow!("Generated out of range random number for row data"))?;
        let values = &rand_row_data.values.as_ref().ok_or_else(|| anyhow!("Error getting cell values"))?;

        let random_album = Album {
            name: values[1].effective_value.as_ref().unwrap().string_value.as_ref().unwrap().to_owned(),
            artist: values[0].effective_value.as_ref().unwrap().string_value.as_ref().unwrap().to_owned(),
            genre: (values[2].effective_value.as_ref().unwrap().string_value.as_ref().unwrap()).to_owned(),
            added_by: (values[3].effective_value.as_ref().unwrap().string_value.as_ref().unwrap()).to_owned(),
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
            .get_by_data_filter(GET_ALBUMS_REQUEST.clone(), DOC_ID)
            .doit()
            .await?;
        self.select_random_album(spreadsheet).await
    }
}

#[cfg(test)]
mod test {
    #[tokio::test]
    async fn test_getting_rows() {}
}
