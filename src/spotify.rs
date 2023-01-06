use crate::albums::Album;

use anyhow::{anyhow, Result};

use rspotify::model::search::SearchResult;
use rspotify::{
    model::{Country, Market, SearchType},
    prelude::*,
    ClientCredsSpotify, Credentials,
};

pub struct Spotify {}

fn album_to_query(album: &Album) -> String {
    format!("{} {}", album.name, album.artist)
}

impl Spotify {
    pub async fn fetch_album_link(album: &Album) -> Result<Option<String>> {
        let creds =
            Credentials::from_env().ok_or_else(|| anyhow!("Unable to get Spotify creds"))?;
        let spotify = ClientCredsSpotify::new(creds);

        spotify.request_token().await?;

        let result = spotify
            .search(
                &album_to_query(album),
                SearchType::Album,
                Some(Market::Country(Country::UnitedStates)),
                None,
                Some(1),
                None,
            )
            .await?;
        match result {
            SearchResult::Albums(page) => {
                if page.items.is_empty() {
                    Ok(None)
                } else {
                    return Ok(Some(
                        page.items[0]
                            .to_owned()
                            .external_urls
                            .get("spotify")
                            .ok_or_else(|| anyhow!("Error getting spotify url"))?
                            .to_owned(),
                    ));
                }
            }
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    #[allow(dead_code)]
    async fn test_getting_rotation() -> Result<()> {
        let album = Album {
            name: "Syro".to_owned(),
            artist: "Aphex Twin".to_owned(),
            genre: "Something".to_owned(),
            added_by: "Accident".to_owned(),
            row: 1,
        };
        println!("{:?}", Spotify::fetch_album_link(&album).await?);
        Ok(())
    }
}
